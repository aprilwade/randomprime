use reader_writer::FourCC;
use structs::{FstEntryFile, GcDisc, Resource, ResourceKind};

use crate::mlvl_wrapper::{MlvlArea, MlvlEditor};

use std::{
    collections::{HashMap, HashSet},
    ops::RangeFrom,
};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
struct ResourceKey<'r>
{
    pak_name: &'r [u8],
    kind: FourCC,
    id: u32,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
struct MreaKey<'r>
{
    pak_name: &'r [u8],
    room_id: u32,
}

type SclyPatch<'r, 's> = dyn FnMut(&mut PatcherState, &mut MlvlArea<'r, '_, '_, '_>) -> Result<(), String> + 's;
pub struct PrimePatcher<'r, 's>
{
    file_patches: HashMap<&'s [u8], Box<dyn FnMut(&mut FstEntryFile<'r>) -> Result<(), String> + 's>>,
    // TODO: Come up with a better data structure for this. A per PAK list of patches, for example.
    resource_patches: Vec<(ResourceKey<'s>, Box<dyn FnMut(&mut Resource<'r>) -> Result<(), String> + 's>)>,
    scly_patches: Vec<(MreaKey<'s>, Vec<Box<SclyPatch<'r, 's>>>)>,

    remove_duplicates: bool,
}

pub struct PatcherState
{
    pub fresh_instance_id_range: RangeFrom<u32>,
}

impl<'r, 's> PrimePatcher<'r, 's>
{
    pub fn new() -> PrimePatcher<'r, 's>
    {
        PrimePatcher {
            file_patches: HashMap::new(),
            resource_patches: Vec::new(),
            scly_patches: Vec::new(),
            remove_duplicates: false,
        }
    }

    pub fn deduplicate_resources(&mut self)
    {
        self.remove_duplicates = true;
    }

    pub fn add_file_patch<F>(&mut self, name: &'s [u8], f: F)
        where F: FnMut(&mut FstEntryFile<'r>) -> Result<(), String> + 's
    {
        self.file_patches.insert(name, Box::new(f));
    }

    pub fn add_resource_patch<F>(
        &mut self,
        (paks, res_id, fourcc): (&'_ [&'s [u8]], u32, FourCC),
        f: F,
    )
        where F: Clone + FnMut(&mut Resource<'r>) -> Result<(), String> + 's
    {
        for pak_name in paks {
            let key = ResourceKey {
                pak_name,
                kind: fourcc,
                id: res_id,
            };
            self.resource_patches.push((key, Box::new(f.clone())));
        }
    }

    pub fn add_scly_patch<F>(&mut self, (pak_name, room_id): (&'s [u8], u32), f: F)
        where F: FnMut(&mut PatcherState, &mut MlvlArea<'r, '_, '_, '_>) -> Result<(), String> + 's
    {
        let key = MreaKey { pak_name, room_id, };
        if let Some((_, v)) = self.scly_patches.iter_mut().find(|p| p.0 == key) {
            v.push(Box::new(f));
        } else {
            self.scly_patches.push((key, vec![Box::new(f)]));
        }
    }

    pub fn run(&mut self, gc_disc: &mut GcDisc<'r>) -> Result<(), String>
    {
        let mut patcher_state = PatcherState {
            fresh_instance_id_range: 0xDEADBABE..
        };

        let files_to_patch = self.file_patches.keys()
            .map(|k| *k)
            .chain(self.scly_patches.iter().map(|p| p.0.pak_name))
            .chain(self.resource_patches.iter().map(|p| p.0.pak_name))
            .collect::<HashSet<_>>();
        let files = gc_disc.file_system_root.dir_files_iter_mut()
            .filter(|(path, _)| files_to_patch.contains(&path[..]));


        for (name, fst_entry) in files {
            if let Some(patch) = self.file_patches.get_mut(&name[..]) {
                fst_entry.guess_kind();
                patch(&mut fst_entry.file_mut().unwrap())?
            }

            let pak_patch_exists = self.resource_patches.iter()
                .map(|p| p.0.pak_name)
                .chain(self.scly_patches.iter().map(|p| p.0.pak_name))
                .any(|n| n == &name[..]);
            if !pak_patch_exists {
                continue;
            }

            fst_entry.guess_kind();
            let pak = match fst_entry.file_mut().unwrap() {
                structs::FstEntryFile::Pak(pak) => pak,
                _ => panic!(),
            };

            // Frequently when patching the scripting for a room, we want to modify both the MREA
            // for that room and the MLVL for the whole region at the same. The borrow checker
            // doesn't allow us to hold mutable references to both at the same time, so create a
            // copy on the stack to modify and then overwrite the canonical MLVL at the end of the
            // PAK.
            let scly_patch_exists = self.scly_patches.iter().any(|p| p.0.pak_name == &name[..]);
            let mut mlvl_editor = if scly_patch_exists {
                let mlvl = pak.resources.iter()
                    .find(|i| i.fourcc() == reader_writer::FourCC::from_bytes(b"MLVL"));
                mlvl.map(|mlvl| MlvlEditor::new(mlvl.kind.as_mlvl().unwrap().into_owned()))
            } else {
                None
            };

            let mut seen_resources = if self.remove_duplicates {
                Some(std::collections::HashSet::<ResourceKey>::new())
            } else {
                None
            };
            let mut cursor = pak.resources.cursor();
            while cursor.peek().is_some() {
                let res_key = ResourceKey {
                    pak_name: &name[..],
                    kind: cursor.peek().unwrap().fourcc(),
                    id: cursor.peek().unwrap().file_id,
                };

                if let Some(seen_resources) = &mut seen_resources {
                    if !seen_resources.insert(res_key) {
                        // We've already inserted this resource, so delete it
                        cursor.remove();
                        continue;
                    }
                }

                let mut cursor = cursor.cursor_advancer();
                if let Some((_, patch)) = self.resource_patches.iter_mut().find(|p| p.0 == res_key) {
                    patch(cursor.value().unwrap())?;
                }

                let mrea_key = MreaKey {
                    pak_name: &name[..],
                    room_id: cursor.peek().unwrap().file_id,
                };
                if let Some((_, patches)) = self.scly_patches.iter_mut().find(|p| p.0 == mrea_key) {
                    let mut mlvl_area = mlvl_editor.as_mut().unwrap().get_area(&mut cursor);
                    for patch in patches.iter_mut() {
                        patch(&mut patcher_state, &mut mlvl_area)?;
                    }
                }

                if cursor.peek().unwrap().fourcc() == b"MLVL".into() && mlvl_editor.is_some() {
                    let mlvl = mlvl_editor.take().unwrap().mlvl;
                    cursor.value().unwrap().kind = ResourceKind::Mlvl(mlvl);
                }
            }
        }
        Ok(())
    }
}
