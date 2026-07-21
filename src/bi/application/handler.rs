use std::sync::Arc;

use crate::bi::infrastructure::repository::PostgresBiRepository;

pub struct BiHandlers {
    pub(crate) repo: Arc<PostgresBiRepository>,
}

impl BiHandlers {
    pub fn new(repo: Arc<PostgresBiRepository>) -> Self {
        Self { repo }
    }
}
