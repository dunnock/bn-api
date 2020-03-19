use crate::functional::base;
use db::models::*;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[actix_rt::test]
    async fn index_org_member() {
        base::settlement_adjustments::index(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn index_admin() {
        base::settlement_adjustments::index(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn index_user() {
        base::settlement_adjustments::index(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn index_org_owner() {
        base::settlement_adjustments::index(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn index_door_person() {
        base::settlement_adjustments::index(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn index_promoter() {
        base::settlement_adjustments::index(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn index_promoter_read_only() {
        base::settlement_adjustments::index(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn index_org_admin() {
        base::settlement_adjustments::index(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn index_box_office() {
        base::settlement_adjustments::index(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::settlement_adjustments::create(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::settlement_adjustments::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::settlement_adjustments::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::settlement_adjustments::create(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::settlement_adjustments::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::settlement_adjustments::create(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::settlement_adjustments::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::settlement_adjustments::create(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::settlement_adjustments::create(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[actix_rt::test]
    async fn destroy_org_member() {
        base::settlement_adjustments::destroy(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn destroy_admin() {
        base::settlement_adjustments::destroy(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn destroy_user() {
        base::settlement_adjustments::destroy(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_owner() {
        base::settlement_adjustments::destroy(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn destroy_door_person() {
        base::settlement_adjustments::destroy(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter() {
        base::settlement_adjustments::destroy(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter_read_only() {
        base::settlement_adjustments::destroy(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_admin() {
        base::settlement_adjustments::destroy(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn destroy_box_office() {
        base::settlement_adjustments::destroy(Roles::OrgBoxOffice, false).await;
    }
}
