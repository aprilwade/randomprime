use structs::{Area, AreaLayerFlags, Dependency, Mlvl, Mrea, SclyLayer, Resource, ResourceSource};
use reader_writer::{CStr, DiffListCursor, FourCC};


use std::collections::HashMap;

pub struct MlvlEditor<'a>
{
    pub mlvl: Mlvl<'a>,
}

pub struct MlvlArea<'a, 'mlvl, 'cursor, 'list>
    where 'a: 'mlvl,
          'a: 'list,
          'list: 'cursor
{
    pub mrea_cursor: &'cursor mut DiffListCursor<'list, ResourceSource<'a>>,
    pub mlvl_area: &'mlvl mut Area<'a>,
    pub layer_flags: &'mlvl mut AreaLayerFlags,
    pub layer_names: &'mlvl mut Vec<CStr<'a>>,
}

impl<'a> MlvlEditor<'a>
{
    pub fn new(mlvl: Mlvl<'a>) -> MlvlEditor<'a>
    {
        MlvlEditor { mlvl }
    }

    pub fn get_area<'s, 'cursor, 'list: 'cursor>(
        &'s mut self,
        mrea_cursor: &'cursor mut DiffListCursor<'list, ResourceSource<'a>>
    )
        -> MlvlArea<'a, 's, 'cursor, 'list>
    {
        assert_eq!(mrea_cursor.peek().unwrap().fourcc(), b"MREA".into());
        let file_id = mrea_cursor.peek().unwrap().file_id;
        let (i, area) = self.mlvl.areas.iter_mut()
            .enumerate()
            .find(|&(_, ref a)| a.mrea == file_id)
            .unwrap();
        MlvlArea {
            mrea_cursor,
            mlvl_area: area,
            layer_flags: self.mlvl.area_layer_flags.as_mut_vec().get_mut(i).unwrap(),
            layer_names: self.mlvl.area_layer_names.mut_names_for_area(i).unwrap(),
        }
    }
}

impl<'a, 'mlvl, 'cursor, 'list> MlvlArea<'a, 'mlvl, 'cursor, 'list>
    where 'a: 'mlvl,
          'a: 'cursor,
          'list: 'cursor,
{
    pub fn mrea_file_id(&mut self) -> u32
    {
        self.mrea_cursor.peek().unwrap().file_id
    }

    pub fn mrea(&mut self) -> &mut Mrea<'a>
    {
        self.mrea_cursor.value().unwrap().kind.as_mrea_mut().unwrap()
    }

    pub fn add_layer(&mut self, name: CStr<'a>)
    {
        // Mark this layer as active
        self.layer_flags.flags |= 1 << self.layer_flags.layer_count;
        self.layer_flags.layer_count += 1;
        self.layer_names.push(name);

        {
            let deps = self.mlvl_area.dependencies.deps.as_mut_vec();
            let index = deps.len() - 1;
            deps.insert(index, vec![].into());
        }

        self.mrea().scly_section_mut().layers.as_mut_vec().push(SclyLayer::new());
    }

    pub fn add_dependencies<I>(&mut self, pickup_resources: &HashMap<(u32, FourCC), Resource<'a>>,
                               layer_num: usize, deps: I)
        where I: Iterator<Item=Dependency>,
    {
        let layers = self.mlvl_area.dependencies.deps.as_mut_vec();
        let iter = deps.filter_map(|dep| {
                if layers.iter().all(|layer| layer.iter().all(|i| *i != dep)) {
                    let res = pickup_resources[&(dep.asset_id, dep.asset_type)].clone();
                    layers[layer_num].as_mut_vec().push(dep);
                    Some(res)
                }  else {
                    None
                }
            });
        self.mrea_cursor.insert_after(iter);
    }
}

