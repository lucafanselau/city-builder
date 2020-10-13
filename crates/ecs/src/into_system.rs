// We are copying bevy a bit here (from the layout and abstraction idea)

use crate::system::System;

pub trait IntoFunctionSystem<Resources, Q> {
    fn into_system(self) -> Box<dyn System>;
}
