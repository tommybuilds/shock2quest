use std::{
    cell::RefCell,
    collections::HashMap,
    io::{prelude::*, SeekFrom},
    rc::Rc,
    time::Duration,
};

use cgmath::{point3, prelude::*, vec3, Point3};
use cgmath::{vec4, Matrix4, Vector2, Vector3, Vector4};
use collision::Aabb3;
use engine::{
    assets::asset_cache::AssetCache,
    scene::{SceneObject, VertexPositionTexture},
    texture::{AnimatedTexture, TextureTrait},
    texture_format::TextureFormat,
};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

use crate::{
    importers::TEXTURE_IMPORTER,
    ss2_bin_header::SystemShock2BinHeader,
    ss2_common::{
        self, read_array_u16, read_bytes, read_i16, read_i32, read_matrix, read_single,
        read_string_with_size, read_u16, read_u32, read_u8, read_vec3,
    },
    util::load_multiple_textures_for_model,
    SCALE_FACTOR,
};

#[derive(FromPrimitive, ToPrimitive, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VhotType {
    Unknown = 0,
    LightSource = 1,
    Anchor = 2,
    Particle1 = 3,
    Particle2 = 4,
    Particle3 = 5,
    Particle4 = 6,
    Particle5 = 7,
    LightSource2 = 8,
}

#[derive(Debug, Clone)]
pub struct Vhot {
    pub vhot_type: VhotType,
    pub point: Vector3<f32>,
}

impl Vhot {
    pub fn read<T: Read + Seek>(reader: &mut T) -> Vhot {
        let vhot_type_num = read_u32(reader);
        let vhot_type = VhotType::from_u32(vhot_type_num).unwrap();

        let point = read_vec3(reader) / SCALE_FACTOR;
        Vhot { vhot_type, point }
    }
}

pub struct SystemShock2ObjectMesh {
    pub header: ObjBinHeader,
    pub version: u32,

    pub bounding_box: Aabb3<f32>,
    pub materials: Vec<SystemShock2MeshMaterial>,
    pub uvs: Vec<Vector2<f32>>,
    pub vertices: Vec<Vector3<f32>>,
    pub vhots: Vec<Vhot>,
    pub polygons: Vec<SystemShock2ObjectPolygon>,
    pub sub_objects: Vec<SubObjectHeader>,
}

pub fn read<T: Read + Seek>(
    reader: &mut T,
    common_header: &SystemShock2BinHeader,
) -> SystemShock2ObjectMesh {
    let header = read_header(reader, common_header);

    let vertices = read_vertices(&header, reader);

    let polygons: Vec<SystemShock2ObjectPolygon> =
        read_polygons(&header, reader, common_header.version);

    let uvs = read_uvs(&header, reader);

    let mut materials = read_materials(&header, reader);

    read_extended_materials(&header, &mut materials, reader, common_header.version);

    let objs = read_sub_objects(&header, reader);

    let vhots = read_vhots(&header, reader);

    let bounding_box = Aabb3::new(header.bbox_min, header.bbox_max);

    SystemShock2ObjectMesh {
        bounding_box,
        materials,
        vertices,
        polygons,
        header,
        uvs,
        vhots,
        sub_objects: objs,
        version: common_header.version,
    }
}

// Converter
pub fn to_scene_objects(
    mesh: &SystemShock2ObjectMesh,
    asset_cache: &mut AssetCache,
) -> Vec<SceneObject> {
    let mut hashToMaterial = HashMap::new();

    let material_len = mesh.materials.len();
    for idx in 0..material_len {
        let temp_material = &mesh.materials[idx];

        hashToMaterial.insert(temp_material.slot_num as u16, temp_material.clone());
    }

    let slot_to_vertices = to_vertices(mesh);

    let test = slot_to_vertices
        .into_iter()
        .collect::<Vec<(u16, Vec<VertexPositionTexture>)>>();

    let mut mesh_objects = test
        .into_iter()
        .filter_map(|(slot, verts)| {
            if verts.is_empty() {
                return None;
            }
            let geometry: Rc<Box<dyn engine::scene::Geometry>> =
                Rc::new(Box::new(engine::scene::mesh::create(verts)));
            let material = hashToMaterial.get(&slot).unwrap();
            let mut tex_path = material.name.to_string();

            let mut texture_type: Box<dyn TextureFormat> = Box::new(engine::texture_format::PCX);

            if tex_path.contains(".gif") {
                texture_type = Box::new(engine::texture_format::GIF);
            }

            // HACK... for broken texture name
            if tex_path.to_ascii_lowercase().contains("soft12.pcx") {
                tex_path = "soft12 .pcx".to_owned();
            }

            let maybe_texture = { asset_cache.get_opt(&TEXTURE_IMPORTER, &tex_path) };

            maybe_texture.as_ref()?;

            let texture = maybe_texture.unwrap();

            let diffuse_texture: Rc<dyn TextureTrait> = {
                let mut animation_frames = load_multiple_textures_for_model(asset_cache, &tex_path);
                if !animation_frames.is_empty() {
                    animation_frames.insert(0, texture);
                    Rc::new(AnimatedTexture::new(
                        animation_frames,
                        Duration::from_millis(200),
                    ))
                } else {
                    texture
                }
            };

            // let texture_descriptor = Rc::new(RefCell::new(
            //     engine::texture_descriptor::FilePathTextureDescriptor::new(
            //         tex_path.clone(),
            //         texture_type,
            //     ),
            // ));

            let mut transparency = material.transparency;

            // HACK: Why isn't GLAS_S01" loading transparency??
            if tex_path.contains("GLAS_S01") {
                transparency = 0.8
            }

            let material = RefCell::new(engine::scene::basic_material::create(
                diffuse_texture,
                material.emissivity,
                transparency,
            ));

            Some(engine::scene::scene_object::SceneObject::create(
                material, geometry,
            ))
        })
        .collect::<Vec<SceneObject>>();

    let vhots = &mesh.vhots;
    let mut vhot_objs = vhots
        .iter()
        .map(|vhot| {
            let geometry = engine::scene::cube::create();
            let material = RefCell::new(engine::scene::color_material::create(vec3(0.0, 0.0, 1.0)));
            let mut scene_obj = SceneObject::create(material, Rc::new(Box::new(geometry)));
            scene_obj.set_local_transform(
                Matrix4::from_translation(vhot.point) * Matrix4::from_scale(0.025),
            );
            scene_obj
        })
        .collect::<Vec<SceneObject>>();

    mesh_objects.append(&mut vhot_objs);
    mesh_objects
}

// Data
#[derive(Debug, Clone)]
pub struct SystemShock2MeshMaterial {
    pub name: String,
    material_type: u8, // TODO: Add real type
    pub slot_num: u8,
    ipal_index: u32, // unused

    color: Vector4<f32>,
    handle: u32,
    uv_scale: f32,

    pub transparency: f32,
    pub emissivity: f32,
}

fn read_material<T: Read>(reader: &mut T) -> SystemShock2MeshMaterial {
    let name = ss2_common::read_string_with_size(reader, 16);
    let material_type = ss2_common::read_u8(reader);
    let slot_num = ss2_common::read_u8(reader);

    let mut color = vec4(0.0, 0.0, 0.0, 0.0);
    let mut ipal_index = 0;
    let mut handle = 0;
    let mut uv_scale = 1.0;
    if material_type == 1
    /* MD_MAT_COLOR */
    {
        let r = ss2_common::read_u8(reader);
        let g = ss2_common::read_u8(reader);
        let b = ss2_common::read_u8(reader);
        let a = ss2_common::read_u8(reader);

        color = vec4(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        );
        ipal_index = ss2_common::read_u32(reader);
    } else if material_type == 0
    /* MD_MAT_TMAP */
    {
        handle = ss2_common::read_u32(reader);
        uv_scale = ss2_common::read_single(reader);
        color = vec4(1.0, 1.0, 1.0, 1.0);
    } else {
        panic!("Unknown material type: {material_type}");
    }

    SystemShock2MeshMaterial {
        name,
        material_type,
        slot_num,
        ipal_index,
        color,
        handle,
        uv_scale,
        emissivity: 0.0,
        transparency: 0.0,
    }
}

pub fn read_materials<T: Read + Seek>(
    header: &ObjBinHeader,
    reader: &mut T,
) -> Vec<SystemShock2MeshMaterial> {
    reader
        .seek(SeekFrom::Start((header.offset_mats) as u64))
        .unwrap();

    let mut materials = Vec::new();
    let len = header.num_mats;

    for _idx in 0..len {
        let material = read_material(reader);
        materials.push(material);
    }

    materials
}

fn build_vertex(
    transform: &Matrix4<f32>,
    vec: Vector3<f32>,
    uv: Vector2<f32>,
) -> VertexPositionTexture {
    let p = point3(vec.x, vec.y, vec.z);
    let p_transformed = transform.transform_point(p);
    let v_transformed = vec3(p_transformed.x, p_transformed.y, p_transformed.z);

    VertexPositionTexture {
        position: v_transformed,
        uv,
    }
}

pub fn to_vertices(mesh: &SystemShock2ObjectMesh) -> HashMap<u16, Vec<VertexPositionTexture>> {
    let polygons = &mesh.polygons;
    let uvs = &mesh.uvs;
    let vertices = &mesh.vertices;

    let mut hashMap = HashMap::new();

    for poly in polygons {
        let indices = &poly.vertex_indices;
        let uv_indices = &poly.uv_indices;
        let slot = poly.slot_index;

        let vec = Vec::new();
        hashMap.entry(slot).or_insert(vec);

        let verts = hashMap.get_mut(&slot).unwrap();

        let len = indices.len();
        let uv_len = uv_indices.len();
        if len > 1 && uv_len > 1 {
            for idx in 1..(len - 1) {
                let transform = get_transform_for_point(mesh, indices[idx]);
                verts.push(build_vertex(
                    &transform,
                    vertices[indices[idx] as usize],
                    uvs[uv_indices[idx] as usize],
                ));
                verts.push(build_vertex(
                    &transform,
                    vertices[indices[idx + 1] as usize],
                    uvs[uv_indices[idx + 1_usize] as usize],
                ));
                verts.push(build_vertex(
                    &transform,
                    vertices[indices[0] as usize],
                    uvs[uv_indices[0] as usize],
                ));
            }
        }
    }

    hashMap
}

fn get_transform_for_point(header: &SystemShock2ObjectMesh, usize: u16) -> Matrix4<f32> {
    // TODO: Improve perf by caching, instead of O(N^2) iteration across sub objects
    for so in &header.sub_objects {
        if usize >= so.point_start && usize < so.point_stop {
            return so.transform;
        }
    }

    Matrix4::identity()
}

#[derive(Debug, Clone)]
pub struct SystemShock2ObjectPolygon {
    pub vertex_indices: Vec<u16>,
    pub uv_indices: Vec<u16>,
    pub slot_index: u16,
}

fn read_polygon<T: Read>(
    _header: &ObjBinHeader,
    reader: &mut T,
    version: u32,
) -> SystemShock2ObjectPolygon {
    let _index = ss2_common::read_u16(reader);
    let slot_index = ss2_common::read_u16(reader);

    let poly_type = ss2_common::read_u8(reader);
    let num_verts = ss2_common::read_u8(reader);

    // Plane info?
    let _norm = ss2_common::read_u16(reader);
    let _d = ss2_common::read_single(reader);

    // Read vert indices
    let vertex_indices = read_array_u16(reader, num_verts as u32);

    // Read normal indices
    let _normal_indices = read_array_u16(reader, num_verts as u32);

    // Read uv indices, maybe
    let mut uvs = vec![];
    if (poly_type & 3) == 3 {
        uvs = read_array_u16(reader, num_verts as u32);
    }

    if version == 4 {
        let _unknown = read_u8(reader);
    }

    SystemShock2ObjectPolygon {
        vertex_indices,
        uv_indices: uvs,
        slot_index,
    }
}

pub fn read_polygons<T: Read + Seek>(
    header: &ObjBinHeader,
    reader: &mut T,
    version: u32,
) -> Vec<SystemShock2ObjectPolygon> {
    let mut ret = Vec::new();

    reader
        .seek(SeekFrom::Start((header.offset_polygons) as u64))
        .unwrap();

    for _idx in 0..header.num_polygons {
        let polygon = read_polygon(header, reader, version);
        ret.push(polygon);
    }

    ret
}

pub fn read_vhots<T: Read + Seek>(header: &ObjBinHeader, reader: &mut T) -> Vec<Vhot> {
    let mut ret = Vec::new();

    if header.num_vhots > 0 {
        reader
            .seek(SeekFrom::Start((header.offset_vhots) as u64))
            .unwrap();

        for _ in 0..header.num_vhots {
            let vhot = Vhot::read(reader);
            ret.push(vhot);
        }
    }
    ret.sort_by(|a, b| a.vhot_type.cmp(&b.vhot_type));
    ret
}

pub fn read_uvs<T: Read + Seek>(header: &ObjBinHeader, reader: &mut T) -> Vec<Vector2<f32>> {
    let mut ret = Vec::new();

    let space = header.offset_vhots - header.offset_uvs;
    let num_uvs = space / (4 /* size of float */ * 2/* 2 floats in vector2 */);

    if num_uvs > 0 {
        reader
            .seek(SeekFrom::Start((header.offset_uvs) as u64))
            .unwrap();

        for _idx in 0..num_uvs {
            let vec = ss2_common::read_vec2(reader);
            ret.push(vec);
        }
    }

    ret
}

fn read_extended_materials<T: Read + Seek>(
    header: &ObjBinHeader,
    materials: &mut Vec<SystemShock2MeshMaterial>,
    reader: &mut T,
    version: u32,
) {
    assert!(version > 3 && header.size_mat_extra >= 8);
    if version > 3 && header.size_mat_extra >= 8 {
        reader
            .seek(SeekFrom::Start((header.offset_mat_extra) as u64))
            .unwrap();
        let remaining_size = (header.size_mat_extra - 8) as usize;

        let len = materials.len();
        for i in 0..len {
            let transparency = read_single(reader);
            let emissivity = read_single(reader);
            materials[i].transparency = transparency;
            materials[i].emissivity = emissivity;
        }

        if remaining_size > 0 {
            let _unk = read_bytes(reader, remaining_size);
        }
    }
}

fn read_vertices<T: Read + Seek>(header: &ObjBinHeader, reader: &mut T) -> Vec<Vector3<f32>> {
    reader
        .seek(SeekFrom::Start((header.offset_verts) as u64))
        .unwrap();

    let mut vertices = Vec::new();

    let len = header.num_verts;
    for idx in 0..len {
        let vertex_position = read_vec3(reader) / SCALE_FACTOR;
        //println!(" - Vertex {idx}: {vertex_position:?}");
        vertices.push(vertex_position);
    }

    vertices
}

#[derive(Debug)]
pub struct SubObjectHeader {
    parent_idx: i32,
    name: String,
    transform: Matrix4<f32>,
    min_range: f32,
    max_range: f32,
    child_sub_obj_idx: i16,
    next_sub_obj_idx: i16,
    point_start: u16,
    point_stop: u16,
}

fn read_sub_objects<T: Read + Seek>(header: &ObjBinHeader, reader: &mut T) -> Vec<SubObjectHeader> {
    reader
        .seek(SeekFrom::Start((header.offset_objs) as u64))
        .unwrap();

    let _obj_size = (header.offset_mats - header.offset_objs) / (header.num_objs as u32);

    let mut objs = Vec::new();
    for _i in 0..header.num_objs {
        let name = read_string_with_size(reader, 8);
        let _obj_type = read_u8(reader);
        let parent_idx = read_i32(reader);
        let min_range = read_single(reader);
        let max_range = read_single(reader);

        // Transfrorm
        let mut decomposed = read_matrix(reader);
        decomposed.disp /= SCALE_FACTOR;
        let transform: Matrix4<f32> = decomposed.into();

        // let rot: Quaternion<f32> = matrix.into();
        // let euler: Euler<Deg<f32>> = cgmath::Euler::from(rot);
        // println!("rot: {:?}", euler);

        // let remainder = obj_size - 8;
        // println!("-remainder: {}", remainder);
        let child_sub_obj_idx = read_i16(reader);
        let next_sub_obj_idx = read_i16(reader);
        let _vhot_start = read_i16(reader);
        let _num_vhots = read_i16(reader);
        let point_start = read_u16(reader);
        let sub_num_points = read_u16(reader);
        let _ = read_bytes(reader, 12);
        // println!("- start point: {point_start} num_points: {sub_num_points}");

        //panic!("done");
        let soh = SubObjectHeader {
            parent_idx,
            child_sub_obj_idx,
            next_sub_obj_idx,
            min_range,
            max_range,
            name,
            transform,
            point_start,
            point_stop: point_start + sub_num_points,
        };
        println!("soh: {:?}", soh);
        objs.push(soh);
    }
    objs
}

pub struct ObjBinHeader {
    bbox_min: Point3<f32>,
    bbox_max: Point3<f32>,
    obj_name: String,
    num_mats: u8,
    num_objs: u8,
    num_polygons: u16,
    num_verts: u16,
    num_vhots: u8,

    offset_mats: u32,
    offset_mat_extra: u32,
    offset_objs: u32,
    mat_flags: u32,
    size_mat_extra: u32,
    offset_polygons: u32,
    offset_verts: u32,
    offset_vhots: u32,
    offset_uvs: u32,
}

pub fn read_header<T: Read>(reader: &mut T, common_header: &SystemShock2BinHeader) -> ObjBinHeader {
    let version = common_header.version;
    let obj_name = ss2_common::read_string_with_size(reader, 8);

    let _sphere_rad = ss2_common::read_single(reader) / SCALE_FACTOR;
    let _max_poly_rad: f32 = ss2_common::read_single(reader) / SCALE_FACTOR;

    let bbox_max_initial = ss2_common::read_point3(reader) / SCALE_FACTOR;
    let bbox_min_initial = ss2_common::read_point3(reader) / SCALE_FACTOR;

    // Because of the tweaks to the coordinate system, there is no guarantee that the
    // provided min/max are actually the min/max - so we need to normalize them.
    let bbox_min = point3(
        bbox_min_initial.x.min(bbox_max_initial.x),
        bbox_min_initial.y.min(bbox_max_initial.y),
        bbox_min_initial.z.min(bbox_max_initial.z),
    );

    let bbox_max = point3(
        bbox_min_initial.x.max(bbox_max_initial.x),
        bbox_min_initial.y.max(bbox_max_initial.y),
        bbox_min_initial.z.max(bbox_max_initial.z),
    );
    let _parent_center = ss2_common::read_vec3(reader) / SCALE_FACTOR;

    let num_polygons = ss2_common::read_u16(reader);
    let num_verts = ss2_common::read_u16(reader);
    let _num_params = ss2_common::read_u16(reader);

    let num_mats = ss2_common::read_u8(reader);
    let _num_vcalls = ss2_common::read_u8(reader);
    let num_vhots = ss2_common::read_u8(reader);
    let num_objs = ss2_common::read_u8(reader);

    let offset_objs = ss2_common::read_u32(reader);
    let offset_mats = ss2_common::read_u32(reader);
    let offset_uvs = ss2_common::read_u32(reader);
    let offset_vhots = ss2_common::read_u32(reader);
    let offset_verts = ss2_common::read_u32(reader);
    let _offset_lights = ss2_common::read_u32(reader);
    let _offset_normals = ss2_common::read_u32(reader);
    let offset_polygons = ss2_common::read_u32(reader);
    let _offset_nodes = ss2_common::read_u32(reader);
    let _model_size = ss2_common::read_u32(reader);

    let mut offset_mat_extra = 0;
    let mut size_mat_extra = 0;
    let mut mat_flags = 0;

    if version > 3 {
        mat_flags = read_u32(reader);
        offset_mat_extra = read_u32(reader);
        size_mat_extra = read_u32(reader);
        assert!(size_mat_extra >= 8);
    }

    ObjBinHeader {
        bbox_min,
        bbox_max,
        obj_name,
        offset_mats,
        offset_objs,
        offset_verts,
        offset_vhots,
        offset_uvs,
        offset_polygons,
        num_mats,
        num_objs,
        num_polygons,
        num_verts,
        num_vhots,
        offset_mat_extra,
        size_mat_extra,
        mat_flags,
    }
}
