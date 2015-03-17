#![allow(dead_code, unused_variables)]
use assets;
use bsp;
use byteorder::{self, LittleEndian, ReadBytesExt};
use std::io::{Cursor, SeekFrom, Seek};
use graphics;
use glium;
use image;
use na;

#[derive(Debug)]
pub enum BspError {
    ByteOrderError(byteorder::Error)
}
impl ::std::error::FromError<byteorder::Error> for BspError {
    fn from_error(e: byteorder::Error) -> BspError {
        BspError::ByteOrderError(e)
    }
}
pub fn import_collision(data: &[u8]) -> Result<bsp::Tree, BspError> {
    let directory = try!(read_directory(data));
    let planes = try!(read_planes(directory.planes));
    let nodes = try!(read_nodes(directory.nodes, &planes));
    let leaves = try!(read_leaves(directory.leaves)); 

    Ok(bsp::Tree {
        brushes: vec![],
        leaves: leaves,
        inodes: nodes,
    })
}

pub fn import_graphics_model(data: &[u8], display: &glium::Display) -> Result<Vec<graphics::Model>, BspError> {
    use std::collections::VecMap;
    use std::collections::vec_map::Entry;

    let directory = try!(read_directory(data));
    let faces = try!(read_faces(directory.faces));
    let vertices = try!(read_vertices(directory.vertices));
    let meshverts = try!(read_meshverts(directory.meshverts));
    let textures = try!(read_textures(directory.textures));

    let mut texmap = VecMap::new(); 

    for face in faces {
        let &mut (ref mut verts, ref mut indices, _) = match texmap.entry(face.texture as usize) {
            Entry::Vacant(v) => {
                println!("{:?}", textures[face.texture as usize].name);
                let tex = assets::load_bin_asset(&(textures[face.texture as usize].name.clone() + ".png")).unwrap();
                let tex = image::load(::std::old_io::BufReader::new(&tex), image::PNG).unwrap();
                let tex = glium::Texture2d::new(display, tex);
                v.insert( ( vec![], vec![], tex ) )
            },
            Entry::Occupied(o) => {
                o.into_mut()
            }
        };

        for meshvert in &meshverts[face.meshvert as usize .. (face.meshvert + face.n_meshverts) as usize] {
            let vert = &vertices[(face.vertex + *meshvert) as usize];
            let idx = verts.len();
            indices.push(idx as u32);
            println!("{:?}", vert.texcoords);
            verts.push(graphics::Vertex {
                position: [vert.position.x, vert.position.z, vert.position.y],
                texcoords: [1.0 - vert.texcoords.x, 1.0 - vert.texcoords.y]
            });
        }
    }

    let program = ::std::sync::Arc::new(glium::Program::from_source(
        &display,
        &assets::load_str_asset("vertex.glsl").unwrap(),
        &assets::load_str_asset("fragment.glsl").unwrap(),
        None
        ).unwrap());

    let mut models = vec![];
    for (_, (verts, indices, tex)) in texmap {
        models.push(graphics::Model {
            mesh: glium::VertexBuffer::new(display, verts),
            indices: glium::IndexBuffer::new(display, glium::index::TrianglesList(indices)),
            program: program.clone(),
            texture: tex
        })
    }

    Ok(models)
}

struct Directory<'a> {
    textures: &'a [u8],
    planes: &'a [u8],
    nodes: &'a [u8],
    leaves: &'a [u8],
    vertices: &'a [u8],
    meshverts: &'a [u8],
    faces: &'a [u8],
}

fn read_directory(data: &[u8]) -> byteorder::Result<Directory> {
    let mut cursor = Cursor::new(data);

    cursor.seek(SeekFrom::Start(8 + 8)).unwrap();
    let textures_offset = try!(cursor.read_u32::<LittleEndian>());
    let textures_len = try!(cursor.read_u32::<LittleEndian>());

    cursor.seek(SeekFrom::Current(0)).unwrap();
    let planes_offset = try!(cursor.read_u32::<LittleEndian>());
    let planes_len = try!(cursor.read_u32::<LittleEndian>());

    cursor.seek(SeekFrom::Current(0)).unwrap();
    let nodes_offset = try!(cursor.read_u32::<LittleEndian>());
    let nodes_len = try!(cursor.read_u32::<LittleEndian>());

    cursor.seek(SeekFrom::Current(0)).unwrap();
    let leaves_offset = try!(cursor.read_u32::<LittleEndian>());
    let leaves_len = try!(cursor.read_u32::<LittleEndian>());

    cursor.seek(SeekFrom::Current(8*5)).unwrap();
    let vertices_offset = try!(cursor.read_u32::<LittleEndian>());
    let vertices_len = try!(cursor.read_u32::<LittleEndian>());

    cursor.seek(SeekFrom::Current(0)).unwrap();
    let meshverts_offset = try!(cursor.read_u32::<LittleEndian>());
    let meshverts_len = try!(cursor.read_u32::<LittleEndian>());

    cursor.seek(SeekFrom::Current(8*1)).unwrap();
    let faces_offset = try!(cursor.read_u32::<LittleEndian>());
    let faces_len = try!(cursor.read_u32::<LittleEndian>());

    Ok(Directory {
        textures: &data[textures_offset as usize .. (textures_offset + textures_len) as usize],
        planes: &data[planes_offset as usize .. (planes_offset + planes_len) as usize],
        nodes: &data[nodes_offset as usize .. (nodes_offset + nodes_len) as usize], 
        leaves: &data[leaves_offset as usize .. (leaves_offset + leaves_len) as usize],
        vertices: &data[vertices_offset as usize .. (vertices_offset + vertices_len) as usize], 
        meshverts: &data[meshverts_offset as usize .. (meshverts_offset + meshverts_len) as usize], 
        faces: &data[faces_offset as usize .. (faces_offset + faces_len) as usize], 
    })
}

fn read_plane(data: &[u8]) -> byteorder::Result<bsp::Plane> {
    let mut cursor = Cursor::new(data);

    let n_x = try!(cursor.read_f32::<LittleEndian>()); 
    let n_y = try!(cursor.read_f32::<LittleEndian>()); 
    let n_z = try!(cursor.read_f32::<LittleEndian>()); 
    let dist = try!(cursor.read_f32::<LittleEndian>()); 

    Ok(bsp::Plane {
        norm: na::Vec3::new(n_x, n_z, n_y),
        dist: dist
    })
}
fn read_planes(data: &[u8]) -> byteorder::Result<Vec<bsp::Plane>> {
    data.chunks(16)
        .map(|chunk| read_plane(chunk))
        .collect()
}


fn read_node(data: &[u8], planes: &[bsp::Plane]) -> byteorder::Result<bsp::InnerNode> {
    let mut cursor = Cursor::new(data);

    let plane_id = try!(cursor.read_i32::<LittleEndian>()); 
    let front = try!(cursor.read_i32::<LittleEndian>()); 
    let back = try!(cursor.read_i32::<LittleEndian>()); 

    Ok(bsp::InnerNode {
        plane: planes[plane_id as usize].clone(),
        pos: front as i32,
        neg: back as i32,
    })
}
fn read_nodes(data: &[u8], planes: &[bsp::Plane]) -> byteorder::Result<Vec<bsp::InnerNode>> {
    data.chunks(36)
        .map(|chunk| read_node(chunk, planes))
        .collect()
}

fn read_leaf(data: &[u8]) -> byteorder::Result<bsp::Leaf> {
    let mut cursor = Cursor::new(data);
    cursor.seek(SeekFrom::Start(40)).unwrap();

    let leafbrush = try!(cursor.read_i32::<LittleEndian>()); 
    let n_leafbrushes = try!(cursor.read_i32::<LittleEndian>()); 
    Ok(bsp::Leaf {
        leafbrush: leafbrush,
        n_leafbrushes: n_leafbrushes
    })
}

fn read_leaves(data: &[u8]) -> byteorder::Result<Vec<bsp::Leaf>> {
    data.chunks(48)
        .map(|chunk| read_leaf(chunk))
        .collect()
}

fn read_meshverts(data: &[u8]) -> byteorder::Result<Vec<i32>> {
    data.chunks(4)
        .map(|chunk| {
            let mut cursor = Cursor::new(chunk);
            cursor.read_i32::<LittleEndian>()
        })
        .collect()
}

struct Model {
    face: i32,
    n_faces: i32,
}

fn read_model(data: &[u8]) -> byteorder::Result<Model> {
    let mut cursor = Cursor::new(data);
    cursor.seek(SeekFrom::Start(28)).unwrap();
    unimplemented!();
    
}

#[derive(Debug)]
struct Face {
    texture: i32,
    vertex: i32,
    n_vertexes: i32,
    meshvert: i32,
    n_meshverts: i32,
}

fn read_face(data: &[u8]) -> byteorder::Result<Face> {
    let mut cursor = Cursor::new(data);
    let texture = try!(cursor.read_i32::<LittleEndian>()); 
    cursor.seek(SeekFrom::Current(8)).unwrap();
    let vertex = try!(cursor.read_i32::<LittleEndian>()); 
    let n_vertexes = try!(cursor.read_i32::<LittleEndian>()); 
    let meshvert = try!(cursor.read_i32::<LittleEndian>()); 
    let n_meshverts = try!(cursor.read_i32::<LittleEndian>()); 

    Ok(Face {
        texture: texture,
        vertex: vertex,
        n_vertexes: n_vertexes,
        meshvert: meshvert,
        n_meshverts: n_meshverts, 
    })
}

fn read_faces(data: &[u8]) -> byteorder::Result<Vec<Face>> {
    data.chunks(104)
        .map(|chunk| read_face(chunk))
        .collect()
}

#[derive(Debug)]
struct Texture {
    name: String, 
    flags: i32,
    contents: i32,
}

fn read_texture(data: &[u8]) -> byteorder::Result<Texture> {
    let mut cursor = Cursor::new(data);
    let name = &data[0..64];
    let namelen = name.iter()
        .position(|&c| c == 0)
        .unwrap_or(name.len());
    let name = String::from_utf8_lossy(&name[..namelen]).to_string();

    let flags = try!(cursor.read_i32::<LittleEndian>());
    let contents = try!(cursor.read_i32::<LittleEndian>());

    Ok(Texture {
        name: name,
        flags: flags,
        contents: contents
    })
}

fn read_textures(data: &[u8]) -> byteorder::Result<Vec<Texture>> {
    data.chunks(72)
        .map(|chunk| read_texture(chunk))
        .collect()
}

struct Vertex {
    position: na::Vec3<f32>,
    texcoords: na::Vec2<f32>,
    normal: na::Vec3<f32>,
}
fn read_vertex(data: &[u8]) -> byteorder::Result<Vertex> {
    let mut cursor = Cursor::new(data);
    let p_x = try!(cursor.read_f32::<LittleEndian>());
    let p_y = try!(cursor.read_f32::<LittleEndian>());
    let p_z = try!(cursor.read_f32::<LittleEndian>());
    
    let t_x = try!(cursor.read_f32::<LittleEndian>());
    let t_y = try!(cursor.read_f32::<LittleEndian>());
    cursor.seek(SeekFrom::Current(4*2)).unwrap();
    let n_x = try!(cursor.read_f32::<LittleEndian>());
    let n_y = try!(cursor.read_f32::<LittleEndian>());
    let n_z = try!(cursor.read_f32::<LittleEndian>());

    Ok(Vertex {
        position: na::Vec3::new(p_x, p_y, p_z),
        texcoords: na::Vec2::new(t_x, t_y),
        normal: na::Vec3::new(n_x, n_y, n_z)
    })
}

fn read_vertices(data: &[u8]) -> byteorder::Result<Vec<Vertex>> {
    data.chunks(44)
        .map(|chunk| read_vertex(chunk))
        .collect()
}
