use resource_info_table::resource_info;

use serde::Deserialize;

use std::fs;

use reader_writer::{
    FourCC,
    Reader,
    Writable,
};
use structs::{res_id, ResId};

use crate::{
    patch_config::PatchConfig,
};

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

pub struct ExternPickupModel {
    pub name: String,
    pub fourcc: FourCC,
    pub ancs: u32,
    pub cmdl: u32,
    pub scale: f32,
    pub bytes: Box<[u8]>,
    pub dependencies: Vec<(u32, FourCC)>,
}

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

impl ExternPickupModel {
    pub fn parse(filename: String) -> Result<Vec<ExternPickupModel>, String> {

        let mut models: Vec<Self> = Vec::<ExternPickupModel>::new();
        
        let _metadata = fs::read_to_string(format!("{}\\meta.json", filename)).expect(format!("Unable to read extern model metadata from '{}'", filename).as_str());
        let metadata: MetadataJson = serde_json::from_str(&_metadata)
            .map_err(|e| format!("Extern Assets metadata.json parse failed: {}", e))?;

        Ok(models)
    }
}
