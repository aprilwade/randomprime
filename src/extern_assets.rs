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
#[serde(rename_all = "camelCase")]
struct ExternPickupModelJson {
    pub ancs: u32,
    pub cmdl: u32,
    pub scale: f32,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct ExternAssetJson {
    pub old_id: u32,
    pub new_id: u32,
    pub dependencies: Vec<ExternAssetDependencyJson>
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct ExternAssetDependencyJson {
    pub fourcc: String,
    pub id: u32,
}

impl ExternPickupModel {
    pub fn parse(filename: String) -> Vec<ExternPickupModel> {

        let mut models: Vec<Self> = Vec::<ExternPickupModel>::new();
        
        let metadata = fs::read_to_string("filename").expect(format!("Unable to read extern model metadata from '{}'", filename).as_str());
        println!("{}", metadata);

        models
    }

    /*
    let data = fs::read("/etc/hosts").expect(format!("Unable to read asset data from '{}'", filename).as_str());
    */
}
