use {
    moka::future::Cache,
    spaghettinuum::interface::identity::Identity,
    spaghettinuum_native::service::resolver::Resolver,
};

pub struct S2sState {
    pub resolver: Resolver,
    pub challenges: Cache<Vec<u8>, Identity>,
    pub tokens: Cache<String, Identity>,
}

impl S2sState {
    pub fn resolver(&self) -> &Resolver {
        return &self.resolver;
    }
}
