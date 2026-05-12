// TODO: use a system to sync world with renderer
/*
pub fn process_event(&mut self, world: &World, event: &WorldEvent) -> Result<()> {
    match *event {
        WorldEvent::Created(..) | WorldEvent::Destroyed(..) => {}
        WorldEvent::EntitySpawn { world: _, entity } => {
            self.proxies
                .insert(entity, SceneProxy::build(&self.device, self)?);
        }
        WorldEvent::EntityUpdate { world: _, entity } => {
            let Some(proxy) = self.proxies.get_mut(&entity) else {
                return Ok(());
            };
            let Some(transform) = world.get::<components::Transform>(entity) else {
                return Ok(());
            };
            proxy.set_model_matrix(transform.matrix());

            if let Some(renderable) = world.get::<components::Renderable>(entity) {
                proxy.set_model(renderable.model.clone());
            }
            if let Some(camera) = world.get::<components::Camera>(entity) {
                proxy.set_camera(transform.view(), camera.projection());
            }
        }
        WorldEvent::EntityDespawn { world: _, entity } => {
            self.proxies.remove(&entity);
        }
    }
    Ok(())
}
*/
