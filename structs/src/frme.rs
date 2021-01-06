use auto_struct_macros::auto_struct;

use reader_writer::{
    CStr, FourCC, LazyArray, Readable, Reader, RoArray, Writable,
};
use reader_writer::generic_array::GenericArray;
use reader_writer::generic_array::typenum:: *;

use crate::ResId;
use crate::res_id::*;

use std::io;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Frme<'r>
{
    pub version: u32,
    pub unknown0: u32,

    #[auto_struct(derive = widgets.iter()
            .filter(|w| w.kind.fourcc() == b"MODL".into())
            .count() as u32
        )]
    model_count: u32,// TODO: derive?
    pub unknown1: u32,

    #[auto_struct(derive = widgets.len() as u32)]
    widget_count: u32,

    #[auto_struct(init = (widget_count as usize, version))]
    pub widgets: LazyArray<'r, FrmeWidget<'r>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct FrmeWidget<'r>
{
    #[auto_struct(args)]
    version: u32,

    #[auto_struct(derive = kind.fourcc())]
    kind_fourcc: FourCC,
    pub name: CStr<'r>,
    pub parent: CStr<'r>,

    pub use_anim_controller: u8,
    pub default_visible: u8,
    pub default_active: u8,
    pub cull_faces: u8,
    pub color: GenericArray<f32, U4>,
    pub model_draw_flags: u32,

    #[auto_struct(init = (kind_fourcc, version))]
    pub kind: FrmeWidgetKind<'r>,

    #[auto_struct(derive = if worker_id.is_some() { 1 } else { 0 })]
    pub is_worker: u8,
    #[auto_struct(init = if is_worker == 1 { Some(()) } else { None })]
    pub worker_id: Option<u16>,

    pub origin: GenericArray<f32, U3>,
    pub basis: GenericArray<f32, U9>,
    pub rotation_center: GenericArray<f32, U3>,
    pub unknown0: u32,
    pub unknown1: u16,
}

#[derive(Clone, Debug)]
pub enum FrmeWidgetKind<'r>
{
    Head,// HWIG
    Base,// BWIG
    Camera(CameraWidget),// CAMR
    Light(LightWidget),// LITE
    Model(ModelWidget),// MODL
    TextPane(TextPaneWidget),// TXPN
    Meter(MeterWidget),// METR
    Energy(EnergyWidget),// ENRG
    Group(GroupWidget),// GRUP
    TableGroup(TableGroupWidget), // TBGP
    Pane(PaneWidget),// PANE
    Slider(SliderWidget),// SLGP
    Image(ImageWidget<'r>), // IMGP
}

impl<'r> FrmeWidgetKind<'r>
{
    pub fn fourcc(&self) -> FourCC
    {
        match self {
            FrmeWidgetKind::Head => b"HWIG".into(),
            FrmeWidgetKind::Base => b"BWIG".into(),
            FrmeWidgetKind::Camera(_) => b"CAMR".into(),
            FrmeWidgetKind::Light(_) => b"LITE".into(),
            FrmeWidgetKind::Model(_) => b"MODL".into(),
            FrmeWidgetKind::TextPane(_) => b"TXPN".into(),
            FrmeWidgetKind::Meter(_) => b"METR".into(),
            FrmeWidgetKind::Energy(_) => b"ENRG".into(),
            FrmeWidgetKind::Group(_) => b"GRUP".into(),
            FrmeWidgetKind::TableGroup(_) => b"TBGP".into(),
            FrmeWidgetKind::Pane(_) => b"PANE".into(),
            FrmeWidgetKind::Slider(_) => b"SLGP".into(),
            FrmeWidgetKind::Image(_) => b"IMGP".into(),
        }
    }
}

impl<'r> Readable<'r> for FrmeWidgetKind<'r>
{
    type Args = (FourCC, u32);
    fn read_from(reader: &mut Reader<'r>, (fourcc, version): Self::Args) -> Self
    {
        if fourcc == b"HWIG".into() {
            FrmeWidgetKind::Head
        } else if fourcc == b"BWIG".into() {
            FrmeWidgetKind::Base
        } else if fourcc == b"CAMR".into() {
            FrmeWidgetKind::Camera(reader.read(()))
        } else if fourcc == b"LITE".into() {
            FrmeWidgetKind::Light(reader.read(()))
        } else if fourcc == b"MODL".into() {
            FrmeWidgetKind::Model(reader.read(()))
        } else if fourcc == b"TXPN".into() {
            FrmeWidgetKind::TextPane(reader.read(version))
        } else if fourcc == b"METR".into() {
            FrmeWidgetKind::Meter(reader.read(()))
        } else if fourcc == b"ENRG".into() {
            FrmeWidgetKind::Energy(reader.read(()))
        } else if fourcc == b"GRUP".into() {
            FrmeWidgetKind::Group(reader.read(()))
        } else if fourcc == b"TBGP".into() {
            FrmeWidgetKind::TableGroup(reader.read(()))
        } else if fourcc == b"PANE".into() {
            FrmeWidgetKind::Pane(reader.read(()))
        } else if fourcc == b"SLGP".into() {
            FrmeWidgetKind::Slider(reader.read(()))
        } else if fourcc == b"IMGP".into() {
            FrmeWidgetKind::Image(reader.read(()))
        } else {
            panic!("Invalid Frme widget fourcc {:?}", fourcc)
        }
    }

    fn size(&self) -> usize
    {
        match self {
            FrmeWidgetKind::Head => 0,
            FrmeWidgetKind::Base => 0,
            FrmeWidgetKind::Camera(widget) => widget.size(),
            FrmeWidgetKind::Light(widget) => widget.size(),
            FrmeWidgetKind::Model(widget) => widget.size(),
            FrmeWidgetKind::TextPane(widget) => widget.size(),
            FrmeWidgetKind::Meter(widget) => widget.size(),
            FrmeWidgetKind::Energy(widget) => widget.size(),
            FrmeWidgetKind::Group(widget) => widget.size(),
            FrmeWidgetKind::TableGroup(widget) => widget.size(),
            FrmeWidgetKind::Pane(widget) => widget.size(),
            FrmeWidgetKind::Slider(widget) => widget.size(),
            FrmeWidgetKind::Image(widget) => widget.size(),
        }
    }
}

impl<'r> Writable for FrmeWidgetKind<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        match self {
            FrmeWidgetKind::Head => Ok(0),
            FrmeWidgetKind::Base => Ok(0),
            FrmeWidgetKind::Camera(widget) => widget.write_to(writer),
            FrmeWidgetKind::Light(widget) => widget.write_to(writer),
            FrmeWidgetKind::Model(widget) => widget.write_to(writer),
            FrmeWidgetKind::TextPane(widget) => widget.write_to(writer),
            FrmeWidgetKind::Meter(widget) => widget.write_to(writer),
            FrmeWidgetKind::Energy(widget) => widget.write_to(writer),
            FrmeWidgetKind::Group(widget) => widget.write_to(writer),
            FrmeWidgetKind::TableGroup(widget) => widget.write_to(writer),
            FrmeWidgetKind::Pane(widget) => widget.write_to(writer),
            FrmeWidgetKind::Slider(widget) => widget.write_to(writer),
            FrmeWidgetKind::Image(widget) => widget.write_to(writer),
        }
    }
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraWidget
{
    #[auto_struct(derive = if perspective_projection.is_some() {
            assert!(orthographic_projection.is_none());
            0
        } else {
            assert!(orthographic_projection.is_some());
            1
        })]
    projection_type: u32,

    #[auto_struct(init = if projection_type == 0 { Some(()) } else { None })]
    pub perspective_projection: Option<GenericArray<f32, U4>>,
    #[auto_struct(init = if projection_type == 1 { Some(()) } else { None })]
    pub orthographic_projection: Option<GenericArray<f32, U6>>,
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct LightWidget
{
    pub light_type: u32,

    pub dist_c: f32,
    pub dist_l: f32,
    pub dist_q: f32,
    pub ang_c: f32,
    pub ang_l: f32,
    pub ang_q: f32,
    pub loaded_idx: u32,

    #[auto_struct(init = if light_type == 0 { Some(()) } else { None })]
    pub cutoff: Option<f32>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct EnergyWidget
{
    pub txtr: ResId<TXTR>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MeterWidget
{
    pub unknown: u8,
    pub no_round_up: u8,
    pub max_capacity: u32,
    pub worker_count: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct GroupWidget
{
    pub default_worker: u16,
    pub unknown: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct TableGroupWidget
{
    pub element_count: u16,
    pub unknown0: u16,
    pub unknown1: u32,
    pub default_selection: u16,
    pub unknown2: u16,
    pub select_wraparound: u8,
    pub unknown3: u8,
    pub unknown4: f32,
    pub unknown5: f32,
    pub unknown6: u8,
    pub unknown7: f32,
    pub unknown8: u16,
    pub unknown9: u16,
    pub unknown10: u16,
    pub unknown11: u16,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct SliderWidget
{
    pub min: f32,
    pub max: f32,
    pub curr: f32,
    pub increment: f32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PaneWidget
{
    pub x_dim: f32,
    pub z_dim: f32,
    pub scale_center: GenericArray<f32, U3>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct TextPaneWidget
{
    #[auto_struct(args)]
    version: u32,

    pub x_dim: f32,
    pub z_dim: f32,
    pub scale_center: GenericArray<f32, U3>,

    pub font: ResId<FONT>,
    pub word_wrap: u8,
    pub horizontal: u8,
    pub justification: u32,
    pub vertical_justification: u32,
    pub fill_color: GenericArray<f32, U4>,
    pub outline_color: GenericArray<f32, U4>,
    pub block_extent: GenericArray<f32, U2>,

    #[auto_struct(init = if version == 1 { Some(()) } else { None })]
    pub jpn_font: Option<ResId<FONT>>,
    #[auto_struct(init = if version == 1 { Some(()) } else { None })]
    pub jpn_point_scale: Option<GenericArray<u32, U2>>,
    // TODO: If Frme::version == 1, then theres three extra fields
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ImageWidget<'r>
{
    pub texture: ResId<TXTR>,
    pub unknown0: u32,
    pub unknown1: u32,

    #[auto_struct(derive = quad_coords.len() as u32)]
    quad_coord_count: u32,
    #[auto_struct(init = (quad_coord_count as usize, ()))]
    pub quad_coords: RoArray<'r, GenericArray<f32, U3>>,

    #[auto_struct(derive = uv_coords.len() as u32)]
    uv_coord_count: u32,
    #[auto_struct(init = (uv_coord_count as usize, ()))]
    pub uv_coords: RoArray<'r, GenericArray<f32, U2>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ModelWidget
{
    pub model: ResId<CMDL>,
    pub blend_mode: u32,
    pub light_mask: u32,
}
