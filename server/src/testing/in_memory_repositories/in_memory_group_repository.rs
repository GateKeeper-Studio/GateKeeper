use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{group::Group, user::User},
        errors::DomainError,
        repositories::group_repository::GroupRepository,
    },
    testing::in_memory_repositories::in_memory_user_repository::InMemoryUserRepository,
    testing::in_memory_unit_of_work::InMemoryRepository,
};

// In-memory GroupRepository for unit tests.
//
// Group entities use the standard draft/data/deleted store. Membership is kept
// in its own (group_id, user_id) sets with the same draft/commit semantics, so
// adds made inside a Unit of Work only become visible after commit_members().
// Tests can seed committed state directly via `memory.data` and `members`.
pub struct InMemoryGroupRepository {
    pub memory: Arc<InMemoryRepository<Uuid, Group>>,
    pub members: Mutex<HashSet<(Uuid, Uuid)>>,
    members_draft: Mutex<HashSet<(Uuid, Uuid)>>,
    members_deleted: Mutex<HashSet<(Uuid, Uuid)>>,
    // Used by find_members to resolve member users; shares the UoW's user store.
    user_repository: Arc<InMemoryUserRepository>,
}

impl InMemoryGroupRepository {
    pub fn new(user_repository: Arc<InMemoryUserRepository>) -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
            members: Mutex::new(HashSet::new()),
            members_draft: Mutex::new(HashSet::new()),
            members_deleted: Mutex::new(HashSet::new()),
            user_repository,
        }
    }

    // Flushes membership drafts/deletions into the committed set. Called by the
    // in-memory Unit of Work alongside the entity-store commit.
    pub fn commit_members(&self) {
        let mut members = self.members.lock().unwrap();
        for m in self.members_draft.lock().unwrap().drain() {
            members.insert(m);
        }
        for m in self.members_deleted.lock().unwrap().drain() {
            members.remove(&m);
        }
    }

    // Effective membership = committed ∪ draft − deleted.
    fn effective_members(&self) -> HashSet<(Uuid, Uuid)> {
        let mut set = self.members.lock().unwrap().clone();
        for m in self.members_draft.lock().unwrap().iter() {
            set.insert(*m);
        }
        for m in self.members_deleted.lock().unwrap().iter() {
            set.remove(m);
        }
        set
    }
}

#[async_trait]
impl GroupRepository for InMemoryGroupRepository {
    async fn find_by_id(&self, group_id: Uuid) -> Result<Option<Group>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.get(&group_id).cloned())
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Group>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        let mut groups: Vec<Group> = data
            .values()
            .filter(|g| g.tenant_id() == tenant_id)
            .cloned()
            .collect();
        // Newest first, matching the DB ordering.
        groups.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
        Ok(groups
            .into_iter()
            .skip(((page - 1) * page_size) as usize)
            .take(page_size as usize)
            .collect())
    }

    async fn quantity_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.values().filter(|g| g.tenant_id() == tenant_id).count() as u32)
    }

    async fn is_group_with_name_exists(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<bool, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .any(|g| g.tenant_id() == tenant_id && g.name() == name))
    }

    async fn find_by_user_id(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Group>, DomainError> {
        let members = self.effective_members();
        let data = self.memory.data.lock().unwrap();
        Ok(members
            .iter()
            .filter(|(_, u)| *u == user_id)
            .filter_map(|(g, _)| data.get(g).cloned())
            .filter(|g| g.tenant_id() == tenant_id)
            .collect())
    }

    async fn add(&self, group: &Group) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(group.id(), group.clone());
        Ok(())
    }

    async fn update(&self, group: &Group) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(group.id(), group.clone());
        Ok(())
    }

    async fn delete_by_id(&self, group_id: Uuid) -> Result<(), DomainError> {
        self.memory.deleted.lock().unwrap().insert(group_id);
        Ok(())
    }

    async fn add_member(&self, group_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        self.members_deleted
            .lock()
            .unwrap()
            .remove(&(group_id, user_id));
        self.members_draft
            .lock()
            .unwrap()
            .insert((group_id, user_id));
        Ok(())
    }

    async fn remove_member(&self, group_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        self.members_draft
            .lock()
            .unwrap()
            .remove(&(group_id, user_id));
        self.members_deleted
            .lock()
            .unwrap()
            .insert((group_id, user_id));
        Ok(())
    }

    async fn is_member(&self, group_id: Uuid, user_id: Uuid) -> Result<bool, DomainError> {
        Ok(self.effective_members().contains(&(group_id, user_id)))
    }

    async fn count_members(&self, group_id: Uuid) -> Result<u32, DomainError> {
        Ok(self
            .effective_members()
            .iter()
            .filter(|(g, _)| *g == group_id)
            .count() as u32)
    }

    async fn find_members(
        &self,
        group_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<User>, DomainError> {
        let member_ids: Vec<Uuid> = self
            .effective_members()
            .iter()
            .filter(|(g, _)| *g == group_id)
            .map(|(_, u)| *u)
            .collect();

        let data = self.user_repository.memory.data.lock().unwrap();
        let mut users: Vec<User> = member_ids
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
