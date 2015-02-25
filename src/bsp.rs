#![allow(dead_code, unused_variables)]

use na;
use na::Dot;
use self::cast::{
    Ray,
    CastResult
};

pub struct Plane {
    pub norm: na::Vec3<f32>,
    pub dist: f32
}
impl Plane {
    pub fn cast_ray(&self, ray: &Ray) -> Option<CastResult> {
        let rn = ray.dir.dot(&self.norm);
        let nn = (self.norm * self.dist - ray.orig.to_vec()).dot(&self.norm);
        let toi = if na::approx_eq(&rn, &0.0) { 
            if na::approx_eq(&nn, &0.0) {
                0.0
            } else {
                // ew early return
                return None;
            }
        } else {
            nn / rn
        };

        if toi >= 0.0 { 
            Some(
                CastResult {
                    toi: toi,
                    norm: self.norm
                }
            ) 
        } else {
            None
        }
    }
}

pub type NodeIndex = usize;

pub enum Node {
    Inner {
        plane: Plane,
        /// Subtree in the same direction as the normal
        pos: NodeIndex,
        /// Subtree against the normal
        neg: NodeIndex,
    },
    Leaf {
        parent: NodeIndex,
        solid: bool,
    }
}

pub struct Tree {
    nodes: Vec<Node>,
    root: NodeIndex
}
impl Tree {
    pub fn contains_point(&self, point: &na::Pnt3<f32>) -> bool {
        self.contains_point_recursive(point, self.root)
    }
    fn contains_point_recursive(&self, point: &na::Pnt3<f32>, nodeidx: NodeIndex) -> bool {
        match self.nodes[nodeidx] {
            Node::Inner { ref plane, pos, neg } => {
                let planept = plane.norm * plane.dist;
                let dir = point.to_vec() - planept; 
                if na::dot(&dir, &plane.norm) > 0.0 {
                    self.contains_point_recursive(point, pos)
                } else {
                    self.contains_point_recursive(point, neg)
                }
            }
            Node::Leaf { solid, .. } => solid,
        }
    }

    pub fn cast_ray(&self, ray: &Ray) -> Option<CastResult> {
        // This check is necessary because the recursive check is "edge-triggered".
        // In other words, it only considers each plane and the line, and does not check the starting point.
        if self.contains_point(&ray.orig) {
            None
        } else {
            self.cast_ray_recursive(ray, self.root)
        }
    }

    fn cast_ray_recursive(&self, ray: &Ray, nodeidx: NodeIndex) -> Option<CastResult> {
        match self.nodes[nodeidx] {
            Node::Inner { ref plane, pos, neg, .. } => {
                let (first, last) = if plane.norm.dot(&ray.dir) > 0.0 {
                    (pos, neg)
                } else {
                    (neg, pos)
                };

                let cast = plane.cast_ray(ray);
                if let Some(cast) = plane.cast_ray(ray) {
                    // we might need to go "through" the plane
                    // check both sides
                    self.cast_ray_recursive(ray, first)
                        .or(self.cast_ray_recursive(ray, last))
                } else {
                    // only need to check one subtree
                    self.cast_ray_recursive(ray, first)
                }
            }
            Node::Leaf { parent, solid } => {
                if solid {
                    let ref parent = self.nodes[parent];
                    if let &Node::Inner { ref plane, .. } = parent {
                        plane.cast_ray(ray)
                    } else {
                        unreachable!()
                    }
                } else {
                    None
                }

            }
        }
    }

}

pub mod cast {
    use na;

    pub struct Ray {
        pub orig: na::Pnt3<f32>,
        pub dir: na::Vec3<f32>
    }

    pub struct CastResult {
        /// Time of impact.
        pub toi: f32,
        /// Normal of what it hit, where it hit.
        pub norm: na::Vec3<f32>,
    }
}

#[cfg(test)]
mod test {
    use na;
    use super::{
        Node,
        Plane,
        Tree
    };
    use super::cast::{
        Ray,
        CastResult
    };

    macro_rules! assert_castresult {
        ($e: expr, $toi: expr, $norm: expr) => {
            if let Some(ref c) = $e {
                if na::approx_eq(&c.toi, &$toi) {
                    ()
                } else {
                    panic!("Wrong TOI: Expected {:?}, got {:?}", $toi, c.toi);
                }

                if na::approx_eq(&c.norm, &$norm) {
                    ()
                } else {
                    panic!("Wrong normal: Expected {:?}, got {:?}", $norm, c.norm);
                }
            } else {
                panic!("Expected a hit, got a miss!")
            }
        }
    }


    fn test_tree() -> Tree {
        Tree {
            nodes: vec![
                Node::Inner {
                    plane: Plane {
                        norm: na::Vec3::new(1.0, 0.0, 0.0),
                        dist: 0.0,
                    },
                    pos: 2,
                    neg: 1,
                },
                Node::Leaf {
                    solid: false,
                    parent: 0,
                },
                Node::Inner {
                    plane: Plane {
                        norm: na::Vec3::new(1.0, 0.0, 0.0),
                        dist: 1.0,
                    },
                    pos: 3,
                    neg: 4,
                },
                Node::Leaf {
                    solid: false,
                    parent: 2,
                },
                Node::Leaf {
                    solid: true,
                    parent: 2,
                }
            ],
            root: 0
        }
    }

    #[test]
    fn plane_raycast() {
        let plane = Plane {
            norm: na::Vec3::new(1.0, 0.0, 0.0),
            dist: 0.0
        };

        //      ->   |
        let r1 = Ray {
            orig: na::Pnt3::new(-1.0, 0.0, 0.0),
            dir: na::Vec3::new(1.0, 0.0, 0.0)
        };
        assert!(plane.cast_ray(&r1).is_some());

        //      <-   |
        let r2 = Ray {
            orig: na::Pnt3::new(-1.0, 0.0, 0.0),
            dir: na::Vec3::new(-1.0, 0.0, 0.0)
        };
        assert!(!plane.cast_ray(&r2).is_some());
    }

    #[test]
    fn bsp_raycast() { 
        let tree = test_tree();

        //      ->   |  |
        let r1 = Ray {
            orig: na::Pnt3::new(-0.5, 0.0, 0.0),
            dir: na::Vec3::new(1.0, 0.0, 0.0)
        };
        assert_castresult!(tree.cast_ray(&r1), 1.5, na::Vec3::new(1.0, 0.0, 0.0));

        //      <-   |  |
        let r2 = Ray {
            orig: na::Pnt3::new(-0.5, 0.0, 0.0),
            dir: na::Vec3::new(-1.0, 0.0, 0.0)
        };
        assert!(!tree.cast_ray(&r2).is_some());
    }

    #[test]
    fn bsp_contains_point() { 
        let tree = test_tree();

        let p1 = na::Pnt3::new(0.5, 0.0, 0.0);
        let p2 = na::Pnt3::new(1.5, 0.0, 0.0);
        assert!(tree.contains_point(&p1));
        assert!(!tree.contains_point(&p2));
    }
}
