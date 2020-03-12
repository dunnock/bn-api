use crate::functional::base;
use bigneon_db::models::*;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[actix_rt::test]
    async fn index_org_member() {
        base::settlements::index(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn index_admin() {
        base::settlements::index(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn index_user() {
        base::settlements::index(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn index_org_owner() {
        base::settlements::index(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn index_door_person() {
        base::settlements::index(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn index_promoter() {
        base::settlements::index(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn index_promoter_read_only() {
        base::settlements::index(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn index_org_admin() {
        base::settlements::index(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn index_box_office() {
        base::settlements::index(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod show_tests {
    use super::*;
    #[actix_rt::test]
    async fn show_org_member() {
        base::settlements::show(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn show_admin() {
        base::settlements::show(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn show_user() {
        base::settlements::show(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn show_org_owner() {
        base::settlements::show(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn show_door_person() {
        base::settlements::show(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn show_promoter() {
        base::settlements::show(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn show_promoter_read_only() {
        base::settlements::show(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn show_org_admin() {
        base::settlements::show(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn show_box_office() {
        base::settlements::show(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[actix_rt::test]
    async fn destroy_org_member() {
        base::settlements::destroy(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn destroy_admin() {
        base::settlements::destroy(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn destroy_user() {
        base::settlements::destroy(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_owner() {
        base::settlements::destroy(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn destroy_door_person() {
        base::settlements::destroy(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter() {
        base::settlements::destroy(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter_read_only() {
        base::settlements::destroy(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_admin() {
        base::settlements::destroy(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn destroy_box_office() {
        base::settlements::destroy(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::settlements::create(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::settlements::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::settlements::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::settlements::create(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::settlements::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::settlements::create(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::settlements::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::settlements::create(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::settlements::create(Roles::OrgBoxOffice, false).await;
    }
}
