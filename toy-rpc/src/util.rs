use std::collections::HashMap;

use crate::service::AsyncHandler;

pub trait ServiceUtil {
    type State;

    fn get_handlers() -> &'static HashMap<&'static str, AsyncHandler<Self::State>>;
}