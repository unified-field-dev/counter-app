use std::sync::Arc;

use valence::{Actor, Valence, ValenceFactory};

#[derive(Clone)]
pub struct BenchValenceFactory;

impl ValenceFactory for BenchValenceFactory {
    fn build(&self, actor_json: &serde_json::Value) -> anyhow::Result<Valence> {
        let actor: Actor = serde_json::from_value(actor_json.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize actor: {}", e))?;
        Ok(Valence::new(valence::DatabaseRouter::global())
            .with_actor(actor)
            .build())
    }
}

pub fn factory_arc() -> Arc<dyn ValenceFactory> {
    Arc::new(BenchValenceFactory)
}
