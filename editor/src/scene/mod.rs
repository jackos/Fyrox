use crate::{
    audio::EffectSelection,
    camera::CameraController,
    interaction::navmesh::{
        data_model::{Navmesh, NavmeshTriangle, NavmeshVertex},
        selection::NavmeshSelection,
    },
    scene::clipboard::Clipboard,
    world::graph::selection::GraphSelection,
    GameEngine,
};
use fyrox::scene::particle_system::ParticleSystem;
use fyrox::scene::pivot::PivotBuilder;
use fyrox::{
    core::{
        math::TriangleDefinition,
        pool::{Handle, Pool},
        visitor::Visitor,
    },
    engine::Engine,
    scene::{base::BaseBuilder, node::Node, Scene},
};
use std::{collections::HashMap, fmt::Write, path::PathBuf};

pub mod clipboard;

#[macro_use]
pub mod commands;

pub struct EditorScene {
    pub path: Option<PathBuf>,
    pub scene: Handle<Scene>,
    // Handle to a root for all editor nodes.
    pub root: Handle<Node>,
    pub selection: Selection,
    pub clipboard: Clipboard,
    pub camera_controller: CameraController,
    pub navmeshes: Pool<Navmesh>,
}

impl EditorScene {
    pub fn from_native_scene(mut scene: Scene, engine: &mut Engine, path: Option<PathBuf>) -> Self {
        let root = PivotBuilder::new(BaseBuilder::new()).build(&mut scene.graph);
        let camera_controller = CameraController::new(&mut scene.graph, root);

        // Prevent physics simulation in while editing scene.
        scene.graph.physics.enabled = false;
        scene.graph.physics2d.enabled = false;

        let mut navmeshes = Pool::new();

        for navmesh in scene.navmeshes.iter() {
            let _ = navmeshes.spawn(Navmesh {
                vertices: navmesh
                    .vertices()
                    .iter()
                    .map(|vertex| NavmeshVertex {
                        position: vertex.position,
                    })
                    .collect(),
                triangles: navmesh
                    .triangles()
                    .iter()
                    .map(|triangle| NavmeshTriangle {
                        a: Handle::new(triangle[0], 1),
                        b: Handle::new(triangle[1], 1),
                        c: Handle::new(triangle[2], 1),
                    })
                    .collect(),
            });
        }

        EditorScene {
            path,
            root,
            camera_controller,
            navmeshes,
            scene: engine.scenes.add(scene),
            selection: Default::default(),
            clipboard: Default::default(),
        }
    }

    pub fn save(&mut self, path: PathBuf, engine: &mut GameEngine) -> Result<String, String> {
        let scene = &mut engine.scenes[self.scene];

        // Validate first.
        let valid = true;
        let mut reason = "Scene is not saved, because validation failed:\n".to_owned();

        if valid {
            self.path = Some(path.clone());

            let editor_root = self.root;
            let (mut pure_scene, _) = scene.clone(&mut |node, _| node != editor_root);

            // Reset state of nodes. For some nodes (such as particles systems) we use scene as preview
            // so before saving scene, we have to reset state of such nodes.
            for node in pure_scene.graph.linear_iter_mut() {
                if let Some(particle_system) = node.cast_mut::<ParticleSystem>() {
                    // Particle system must not save generated vertices.
                    particle_system.clear_particles();
                }
            }

            pure_scene.navmeshes.clear();

            for navmesh in self.navmeshes.iter() {
                // Sparse-to-dense mapping - handle to index.
                let mut vertex_map = HashMap::new();

                let vertices = navmesh
                    .vertices
                    .pair_iter()
                    .enumerate()
                    .map(|(i, (handle, vertex))| {
                        vertex_map.insert(handle, i);
                        vertex.position
                    })
                    .collect::<Vec<_>>();

                let triangles = navmesh
                    .triangles
                    .iter()
                    .map(|triangle| {
                        TriangleDefinition([
                            vertex_map[&triangle.a] as u32,
                            vertex_map[&triangle.b] as u32,
                            vertex_map[&triangle.c] as u32,
                        ])
                    })
                    .collect::<Vec<_>>();

                pure_scene
                    .navmeshes
                    .add(fyrox::utils::navmesh::Navmesh::new(&triangles, &vertices));
            }

            let mut visitor = Visitor::new();
            pure_scene.save("Scene", &mut visitor).unwrap();
            if let Err(e) = visitor.save_binary(&path) {
                Err(format!("Failed to save scene! Reason: {}", e))
            } else {
                Ok(format!("Scene {} was successfully saved!", path.display()))
            }
        } else {
            writeln!(&mut reason, "\nPlease fix errors and try again.").unwrap();

            Err(reason)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    None,
    SoundContext,
    Graph(GraphSelection),
    Navmesh(NavmeshSelection),
    Effect(EffectSelection),
}

impl Default for Selection {
    fn default() -> Self {
        Self::None
    }
}

impl Selection {
    pub fn is_empty(&self) -> bool {
        match self {
            Selection::None => true,
            Selection::Graph(graph) => graph.is_empty(),
            Selection::Navmesh(navmesh) => navmesh.is_empty(),
            Selection::SoundContext => false,
            Selection::Effect(effect) => effect.is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Selection::None => 0,
            Selection::Graph(graph) => graph.len(),
            Selection::Navmesh(navmesh) => navmesh.len(),
            Selection::SoundContext => 1,
            Selection::Effect(effect) => effect.len(),
        }
    }

    pub fn is_single_selection(&self) -> bool {
        self.len() == 1
    }
}

#[macro_export]
macro_rules! define_vec_add_remove_commands {
    (struct $add_name:ident, $remove_name:ident<$model_ty:ty, $value_ty:ty> ($self:ident, $context:ident)$get_container:block) => {
        #[derive(Debug)]
        pub struct $add_name {
            pub handle: Handle<$model_ty>,
            pub value: $value_ty,
        }

        impl Command for $add_name {
            fn name(&mut self, _: &SceneContext) -> String {
                stringify!($add_name).to_owned()
            }

            fn execute(&mut $self, $context: &mut SceneContext) {
                $get_container.push(std::mem::take(&mut $self.value));
            }

            fn revert(&mut $self, $context: &mut SceneContext) {
                $self.value = $get_container.pop().unwrap();
            }
        }

        #[derive(Debug)]
        pub struct $remove_name {
            pub handle: Handle<$model_ty>,
            pub index: usize,
            pub value: Option<$value_ty>,
        }

        impl Command for $remove_name {
            fn name(&mut self, _: &SceneContext) -> String {
                stringify!($remove_name).to_owned()
            }

            fn execute(&mut $self, $context: &mut SceneContext) {
                $self.value = Some($get_container.remove($self.index));
            }

            fn revert(&mut $self, $context: &mut SceneContext) {
                $get_container.insert($self.index, $self.value.take().unwrap());
            }
        }
    };
}
