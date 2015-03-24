use Game;
use glium;
use glium::Surface;
use map::GraphicsMap;
use na;
use std::sync::Arc;
use std::default::Default;

pub mod wavefront;
pub mod hud;

#[derive(Copy)]
pub struct Vertex {
    pub position: [f32; 3], 
    pub texcoords: [f32; 2] 
}
implement_vertex!(Vertex, position, texcoords);

pub struct Model {
    pub mesh: glium::VertexBuffer<Vertex>,
    pub indices: glium::IndexBuffer, 
    pub program: Arc<glium::Program>, 
    pub texture: glium::Texture2d,
}

pub struct View {
    pub w2s: na::Mat4<f32>,
}

pub struct Scene {
    pub map: GraphicsMap
}

pub fn draw_scene<S: glium::Surface>(surface: &mut S,
                                     scene: &Scene,
                                     view: &View) {
    draw_map(surface, &scene.map, view);
}

fn draw_map<S: glium::Surface>(surface: &mut S, map: &GraphicsMap, view: &View) {
    let mut drawparams: glium::DrawParameters = Default::default();
    drawparams.depth_test = glium::DepthTest::IfLess;
    drawparams.depth_write = true;

    for face in &map.faces {
        let color = &map.textures[face.texture as usize];
        let colorsamp = glium::uniforms::Sampler(color, Default::default());
        if face.lightmap >= 0 {
            let lightmap = &map.lightmaps[face.lightmap as usize];

            let uniforms = uniform! { 
                transform: *(view.w2s).as_array(),
                color: colorsamp,
                lightmap: lightmap
            };
            surface.draw(&map.vertices,
                       &map.indices.slice(face.index_start as usize, face.index_count as usize).unwrap(),
                       &map.shaders[0],
                       &uniforms,
                       &drawparams).unwrap();
        } else {
    //        println!("Skipping un-lightmapped face...");
        }
    }
}

