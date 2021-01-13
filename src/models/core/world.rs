use std::rc::Rc;

use cgmath::{Vector3, InnerSpace};
use noise::{OpenSimplex};

use super::{block_type::BlockType, chunk::Chunk, coord_map::CoordMap, face::Face};

#[derive(Clone)]
pub struct World {
    chunks: CoordMap<Chunk>,
    render_distance: u32,
    simplex: Rc<OpenSimplex>,
    player_chunk_x: i32,
    player_chunk_z: i32,
    mesh: Vec<Rc<Vec<f32>>>
}

// handles world block data and rendering
impl World {
    pub fn new(render_distance: u32) -> World {
        let chunks = CoordMap::new();
        let simplex = Rc::new(OpenSimplex::new());
        
        World { chunks, render_distance, simplex, player_chunk_x: 0, player_chunk_z: 0, mesh: vec![] }
    }

    // pub fn get_meshes(&self) -> Vec<&Vec<f32>> {
    //     let mut mesh = Vec::new();
    //     for z_axis in self.chunks.iter() {
    //         for x_axis in z_axis.1.iter() {
    //             mesh.push(&x_axis.1.mesh);
    //         }
    //     }
    //     mesh
    // }

    pub fn get_world_mesh_from_perspective(&mut self, player_x: i32, player_z: i32, force: bool) -> &Vec<Rc<Vec<f32>>> {
        let player_chunk_x = player_x / 16;
        let player_chunk_z = player_z / 16;
        if !force 
            && self.mesh.len() > 0 
            && self.player_chunk_x == player_chunk_x 
            && self.player_chunk_z == player_chunk_z {
            return &self.mesh
        }

        self.recalculate_mesh_from_perspective(player_chunk_x, player_chunk_z);

        self.player_chunk_x = player_chunk_x;
        self.player_chunk_z = player_chunk_z;
        
        &self.mesh
    }

    pub fn recalculate_mesh_from_perspective(&mut self, player_chunk_x: i32, player_chunk_z: i32) {
        let mut meshes = Vec::new();
        for x in 0..self.render_distance * 2 {
            let x = (x as i32) - (self.render_distance as i32) + player_chunk_x;
            for z in 0..self.render_distance * 2 {
                let z = (z as i32) - (self.render_distance as i32) + player_chunk_z;
                if (((player_chunk_x - x).pow(2) + (player_chunk_z - z).pow(2)) as f32).sqrt() > self.render_distance as f32 {
                    continue;
                }

                let chunk = self.get_or_insert_chunk(x, z);
                meshes.push(chunk.mesh.clone());
            }
        }

        self.mesh = meshes;
    }

    pub fn get_or_insert_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> &Chunk {
        match self.chunks.contains(chunk_x, chunk_z) {
            true => self.chunks.get(chunk_x, chunk_z).unwrap(),
            false => {
                let c = Chunk::new(chunk_x, chunk_z, self.simplex.clone());
                self.chunks.insert(chunk_x, chunk_z, c);
                self.chunks.get(chunk_x, chunk_z).unwrap()
            }
        }
    }

    pub fn get_chunk_mut(&mut self, chunk_x: i32, chunk_z: i32) -> Option<&mut Chunk> {
        match self.chunks.contains(chunk_x, chunk_z) {
            true => self.chunks.get_mut(chunk_x, chunk_z),
            false => None
        }
    }

    pub fn get_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<&Chunk> {
        self.chunks.get(chunk_x, chunk_z)
    }

    pub fn air_at(&self, world_x: i32, world_y: i32, world_z: i32) -> bool {
        if world_y < 0 {
            return false;
        }

        let (chunk_x, chunk_z, local_x, local_z) = self.localize_coords_to_chunk(world_x, world_z);
        //let instant = std::time::Instant::now();
        let chunk = self.get_chunk(chunk_x, chunk_z);
        //println!("Took {:?} to fetch chunk", instant.elapsed());
        match chunk {
            Some(chunk) => chunk.block_at(local_x, world_y as usize, local_z) == BlockType::Air, 
            None => true
        } 
    }

    pub fn get_block(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<BlockType> {
        let (chunk_x, chunk_z, local_x, local_z) = self.localize_coords_to_chunk(world_x, world_z);
        let chunk = self.get_chunk(chunk_x, chunk_z);
        if chunk.is_none() || world_y < 0 {
            return None
        }

        let result = Some(chunk.unwrap().block_at(local_x, world_y as usize, local_z));
        result
    }

    pub fn highest_in_column(&self, world_x: i32, world_z: i32) -> Option<usize> {
        let (chunk_x, chunk_z, local_x, local_z) = self.localize_coords_to_chunk(world_x, world_z);
        let chunk = self.get_chunk(chunk_x, chunk_z);
        if chunk.is_none() {
            return None
        }

        Some(chunk.unwrap().highest_in_column(local_x, local_z))
    }

    pub fn highest_in_column_from_y(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<usize> {
        let (chunk_x, chunk_z, local_x, local_z) = self.localize_coords_to_chunk(world_x, world_z);
        let chunk = self.get_chunk(chunk_x, chunk_z);
        if chunk.is_none() {
            return None
        }

        Some(chunk.unwrap().highest_in_column_from_y(local_x, world_y as usize, local_z)) 
    }

    pub fn set_block(&mut self, world_x: i32, world_y: i32, world_z: i32, block: BlockType) {
        let (chunk_x, chunk_z, local_x, local_z) = self.localize_coords_to_chunk(world_x, world_z);
        let chunk = self.get_chunk_mut(chunk_x, chunk_z);
        chunk.unwrap().set_block(local_x, world_y as usize, local_z, block);
    }

    fn localize_coords_to_chunk(&self, world_x: i32, world_z: i32) -> (i32, i32, usize, usize) {
        let mut chunk_x = (world_x + if world_x < 0 { 1 } else { 0 }) / 16;
        if world_x < 0 {
            chunk_x -= 1;
        }

        let mut chunk_z = (world_z + if world_z < 0 { 1 } else { 0 }) / 16;
        if world_z < 0 { 
            chunk_z -= 1;
        }

        let local_x = ((chunk_x.abs() * 16 + world_x) % 16).abs() as usize;
        let local_z = ((chunk_z.abs() * 16 + world_z) % 16).abs() as usize;
        (chunk_x, chunk_z, local_x, local_z)
    }

    pub fn raymarch_block(&mut self, position: &Vector3<f32>, direction: &Vector3<f32>) -> Option<((i32, i32, i32), Option<Face>)> {
        let mut check_position = *position;
        let dir: Vector3<f32> = *direction / 10.0;
        let mut range = 250;

        let mut result = Vec::new();
        loop {
            check_position = check_position + dir;
            let x = check_position.x.round() as i32;
            let y = check_position.y.round() as i32;
            let z = check_position.z.round() as i32;
            result.push((x, y, z));

            let block = self.get_block(x, y, z);
            if let Some(block) = block {
                if block != BlockType::Air {
                    let vector = (*position - (check_position - dir)).normalize();
                    let abs_x = vector.x.abs();
                    let abs_y = vector.y.abs();
                    let abs_z = vector.z.abs();
                    let mut face = None;

                    let mut face_is_x = false;
                    // get cube face from ray direction
                    // negated ray is on x-axis
                    let sign = signum(vector.x);
                    if self.air_at(x + sign, y, z) {
                        face = if vector.x > 0.0 {
                            Some(Face::Right)
                        } else {
                            Some(Face::Left)
                        };
                        face_is_x = true;
                    } 
                    
                    if face.is_none() || abs_y > abs_x { 
                        // negated ray is on y-axis
                        let sign = signum(vector.y);
                        if self.air_at(x, y + sign, z) {
                            face = if vector.y > 0.0 {
                                Some(Face::Top)
                            } else {
                                Some(Face::Bottom)
                            };
                            face_is_x = false;
                        }                        
                    } 
                        
                    let sign = signum(vector.z);
                    if face.is_none() || if face_is_x { abs_z > abs_x } else { abs_z > abs_y } {
                        // negated ray is on z-axis
                        //let sign = signum(vector.z);
                        if self.air_at(x, y, z + sign) {
                            face = if vector.z > 0.0 {
                                Some(Face::Back)
                            } else {
                                Some(Face::Front)
                            }
                        }
                    }

                    return Some(((x, y, z), face));
                }
            }

            if range == 0 {
                return None;
            }
            range = range - 1;
        }
    }
}

fn signum(n: f32) -> i32 {
    if n > 0.0 {
        1
    } else {
        -1
    }
}