use serde::Deserialize;
use std::{fs::{self, File}, io::{self, Read}, path::PathBuf};
use std::collections::{HashMap, HashSet};
use reader_writer::FourCC;

/* Public Structs */
#[derive(Debug, Clone)]
pub struct ExternPickupModel {
    pub ancs: u32,
    pub cmdl: u32,
    pub scale: f32,
    pub dependencies: Vec<(u32, FourCC)>,
}

#[derive(Debug, Clone)]
pub struct ExternAsset {
    pub fourcc: FourCC,
    pub bytes: Vec<u8>,
}

/* Structs for modeling JSON format */

#[derive(Deserialize, Debug, Default, Clone)]
struct ExternPickupModelJson {
    pub ancs: u32,
    pub cmdl: u32,
    pub scale: f32,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct ExternAssetJson {
    pub old_id: u32,
    pub new_id: u32,
    pub dependencies: Vec<ExternAssetDependencyJson>
}

#[derive(Deserialize, Debug, Default, Clone)]
struct ExternAssetDependencyJson {
    #[serde(alias = "type")]
    pub fourcc: String,
    pub id: u32,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct MetadataJson {
    pub items: HashMap<String, ExternPickupModelJson>,
    pub new_assets: Vec<ExternAssetJson>,
}

fn parse_dir(dir: &String) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = fs::read_dir(dir)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
        files.sort();
    Ok(files)
}

impl ExternPickupModel {
    pub fn parse(dir: &String) -> Result<(HashMap<String, Self>, HashMap<u32, ExternAsset>), String> {
        // Get file list in dir
        let files = parse_dir(dir)
            .map_err(|e| format!("Extern Assets dir parse failed: {}", e))?;
    
        // Deserialize JSON
        let _metadata = fs::read_to_string(format!("{}\\meta.json", dir)).expect(format!("Unable to read extern model metadata from '{}'", dir).as_str());
        let metadata: MetadataJson = serde_json::from_str(&_metadata)
            .map_err(|e| format!("Extern Assets metadata.json parse failed: {}", e))?;

        // Parse model info
        let mut models: HashMap<String, Self> = HashMap::new();
        for (name, model) in metadata.items.iter() {
            // Collect all dependencies for this model
            let mut dependencies: Vec<(u32, FourCC)> = Vec::new();
            let mut deps: HashSet<u32> = HashSet::new();
            dependencies.push((model.ancs, FourCC::from_bytes(b"ANCS")));
            deps.insert(model.ancs.clone());
            dependencies.push((model.cmdl, FourCC::from_bytes(b"CMDL")));
            deps.insert(model.cmdl.clone());
            let mut added = true;
            while added {
                added = false;
                for asset in metadata.new_assets.iter() {
                    if deps.contains(&asset.new_id) {
                        for dep in &asset.dependencies {
                            if deps.contains(&dep.id) {
                                continue;
                            }
                            let fourcc = dep.fourcc.as_bytes();
                            let fourcc: [u8;4] = [fourcc[0], fourcc[1], fourcc[2], fourcc[3]];
                            let fourcc = FourCC::from_bytes(&fourcc);

                            dependencies.push((dep.id, fourcc));
                            deps.insert(dep.id);
                            added = true;
                        }
                    }
                }
            }

            // Add model to list of availible models
            models.insert(
                name.to_string(),
                ExternPickupModel {
                    ancs: model.ancs,
                    cmdl: model.cmdl,
                    scale: model.scale,
                    dependencies,
                }
            );
        }

        // Asset ids required
        let mut ids_to_find: HashSet<u32> = HashSet::new();
        for (_, model) in models.iter() {
            ids_to_find.insert(model.ancs.clone());
            ids_to_find.insert(model.cmdl.clone());
            for (dep, _) in model.dependencies.iter() {
                ids_to_find.insert(dep.clone());
            }
        }

        // Parse asset data
        let mut assets: HashMap<u32, ExternAsset> = HashMap::new();
        for id in ids_to_find {
            // Find the file which corresponds to this id
            let mut filename = None;
            let mut found = false;
            for file in &files {
                if file.to_str().unwrap().to_string().contains(&format!("{}", id))
                {
                    found = true;
                    filename = Some(file);
                    break;
                }
            }
            if !found {
                panic!("Failed to find file corresponding to asset id {}", id)
            }
            let filename = filename.unwrap();
            // Derrive FourCC from file extension
            // (I dislike Rust; This is just for parsing 4 letters)
            let fourcc = filename.clone();
            let fourcc = fourcc.to_str().unwrap();
            let fourcc = fourcc.split(".");
            let fourcc: Vec<&str> = fourcc.collect();
            if fourcc.len() < 2 {
                panic!("Extern asset, unexpected asset filename format");
            }
            let fourcc = fourcc[fourcc.len() - 1];
            let fourcc = fourcc.as_bytes();
            let fourcc: [u8;4] = [fourcc[0], fourcc[1], fourcc[2], fourcc[3]];
            let fourcc = FourCC::from_bytes(&fourcc);

            // Read file contents to RAM
            let mut file = File::open(&filename).expect("no file found");
            let metadata = fs::metadata(&filename).expect("unable to read metadata");
            let mut bytes = vec![0; metadata.len() as usize];
            file.read(&mut bytes).expect("buffer overflow");

            assets.insert(
                id,
                ExternAsset {
                    fourcc,
                    bytes,
                }
            );
        }

        Ok((models, assets))
    }
}
