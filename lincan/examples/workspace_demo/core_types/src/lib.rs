pub trait Entity {
    fn id(&self) -> u64;
}

pub trait Named: Entity {
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub enum Role {
    Admin,
    User,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Clone, Copy)]
pub struct UserId(pub u64);

#[derive(Debug, Clone, Copy)]
pub struct UserMarker;

impl Entity for User {
    fn id(&self) -> u64 {
        self.id
    }
}

impl Named for User {
    fn name(&self) -> &str {
        &self.name
    }
}

impl User {
    pub fn user_id(&self) -> UserId {
        UserId(self.id)
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.role, Role::Admin)
    }

    pub fn tag(&self) -> String {
        let uid = self.user_id();
        format!("{}#{}", self.name(), uid.0)
    }

    pub fn report_line(&self) -> String {
        self.tag()
    }
}

pub fn display_name(entity: &impl Named) -> String {
    format!("{}#{}", entity.name(), entity.id())
}
