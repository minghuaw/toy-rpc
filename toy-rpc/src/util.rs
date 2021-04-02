use std::collections::HashMap;

use crate::service::AsyncHandler;

pub trait RegisterService {
    fn handlers() -> &'static HashMap<&'static str, AsyncHandler<Self>>;
    fn default_name() -> &'static str;
}