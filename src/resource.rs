use crate::StandardVersionedIndexId;

pub enum ResourceKind {
    Event,
    SceneComponent,
    EntityComponent,
    ViewportComponent,
}

type ResourceId = StandardVersionedIndexId<8>;

pub trait Resource {
    fn kind() -> ResourceKind;
}

pub struct SimpleResourceStorage {
}
