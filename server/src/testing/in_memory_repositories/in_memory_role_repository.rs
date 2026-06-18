use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{role::Role, user::User},
        errors::DomainError,
        repositories::role_repository::RoleRepository,
    },
    testing::in_memory_repositories::in_memory_user_repository::InMemoryUserRepository,
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryRoleRepository {
    pub memory: Arc<InMemoryRepository<Uuid, Role>>,
    pub assignments: Mutex<HashSet<(Uuid, Uuid)>>,
    assignments_draft: Mutex<HashSet<(Uuid, Uuid)>>,
    assignments_deleted: Mutex<HashSet<(Uuid, Uuid)>>,
    user_repository: Arc<InMemoryUserRepository>,
}

impl InMemoryRoleRepository {
    pub fn new(user_repository: Arc<InMemoryUserRepository>) -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
            assignments: Mutex::new(HashSet::new()),
            assignments_draft: Mutex::new(HashSet::new()),
            assignments_deleted: Mutex::new(HashSet::new()),
            user_repository,
        }
    }

    pub fn commit_assignments(&self) {
        let mut assignments = self.assignments.lock().unwrap();
        for a in self.assignments_draft.lock().unwrap().drain() {
            assignments.insert(a);
        }
        for a in self.assignments_deleted.lock().unwrap().drain() {
            assignments.remove(&a);
        }
    }

    fn effective_assignments(&self) -> HashSet<(Uuid, Uuid)> {
        let mut set = self.assignments.lock().unwrap().clone();
        for a in self.assignments_draft.lock().unwrap().iter() {
            set.insert(*a);
        }
        for a in self.assignments_deleted.lock().unwrap().iter() {
            set.remove(a);
        }
        set
    }
}

#[async_trait]
impl RoleRepository for InMemoryRoleRepository {
    async fn find_by_id(&self, role_id: Uuid) -> Result<Option<Role>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.get(&role_id).cloned())
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Role>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        let mut roles: Vec<Role> = data
            .values()
            .filter(|r| r.tenant_id() == tenant_id)
            .cloned()
            .collect();
        roles.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
        Ok(roles
            .into_iter()
            .skip(((page - 1) * page_size) as usize)
            .take(page_size as usize)
            .collect())
    }

    async fn quantity_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.values().filter(|r| r.tenant_id() == tenant_id).count() as u32)
    }

    async fn is_role_with_name_exists(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<bool, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .any(|r| r.tenant_id() == tenant_id && r.name() == name))
    }

    async fn find_by_user_id(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Role>, DomainError> {
        let assignments = self.effective_assignments();
        let data = self.memory.data.lock().unwrap();
        Ok(assignments
            .iter()
            .filter(|(_, u)| *u == user_id)
            .filter_map(|(r, _)| data.get(r).cloned())
            .filter(|r| r.tenant_id() == tenant_id)
            .collect())
    }

    async fn add(&self, role: &Role) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(role.id(), role.clone());
        Ok(())
    }

    async fn update(&self, role: &Role) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(role.id(), role.clone());
        Ok(())
    }

    async fn delete_by_id(&self, role_id: Uuid) -> Result<(), DomainError> {
        self.memory.deleted.lock().unwrap().insert(role_id);
        Ok(())
    }

    async fn assign_user(&self, role_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        self.assignments_deleted
            .lock()
            .unwrap()
            .remove(&(role_id, user_id));
        self.assignments_draft
            .lock()
            .unwrap()
            .insert((role_id, user_id));
        Ok(())
    }

    async fn unassign_user(&self, role_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        self.assignments_draft
            .lock()
            .unwrap()
            .remove(&(role_id, user_id));
        self.assignments_deleted
            .lock()
            .unwrap()
            .insert((role_id, user_id));
        Ok(())
    }

    async fn is_user_assigned(&self, role_id: Uuid, user_id: Uuid) -> Result<bool, DomainError> {
        Ok(self.effective_assignments().contains(&(role_id, user_id)))
    }

    async fn count_assigned_users(&self, role_id: Uuid) -> Result<u32, DomainError> {
        Ok(self
            .effective_assignments()
            .iter()
            .filter(|(r, _)| *r == role_id)
            .count() as u32)
    }

    async fn find_assigned_users(
        &self,
        role_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<User>, DomainError> {
        let user_ids: Vec<Uuid> = self
            .effective_assignments()
            .iter()
            .filter(|(r, _)| *r == role_id)
            .map(|(_, u)| *u)
            .collect();

        let data = self.user_repository.memory.data.lock().unwrap();
        let mut users: Vec<User> = user_ids
            .iter()
            .filter_map(|u| data.get(u).cloned())
            .collect();
        users.sort_by(|a, b| b.created_at().cmp(&a.created_at()));

        Ok(users
            .into_iter()
            .skip(((page - 1) * page_size) as usize)
            .take(page_size as usize)
            .collect())
    }
}
