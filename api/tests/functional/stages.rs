use crate::functional::base;
use db::models::*;

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::stages::create(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::stages::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::stages::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::stages::create(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::stages::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::stages::create(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::stages::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::stages::create(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::stages::create(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_org_member() {
        base::stages::update(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn update_admin() {
        base::stages::update(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_user() {
        base::stages::update(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn update_org_owner() {
        base::stages::update(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn update_door_person() {
        base::stages::update(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter() {
        base::stages::update(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter_read_only() {
        base::stages::update(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn update_org_admin() {
        base::stages::update(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn update_box_office() {
        base::stages::update(Roles::OrgBoxOffice, false).await;
    }
}
