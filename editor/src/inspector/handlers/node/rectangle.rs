use crate::{
    inspector::handlers::node::base::handle_base_property_changed, make_command,
    scene::commands::rectangle::*, SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{dim2::rectangle::Rectangle, node::Node},
};

pub fn handle_rectangle_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
) -> Option<SceneCommand> {
    if node.is_rectangle() {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                Rectangle::TEXTURE => {
                    make_command!(SetRectangleTextureCommand, handle, value)
                }
                Rectangle::COLOR => {
                    make_command!(SetRectangleColorCommand, handle, value)
                }
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                Rectangle::BASE => handle_base_property_changed(inner, handle, node),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
