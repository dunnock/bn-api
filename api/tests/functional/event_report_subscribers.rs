use crate::functional::base;
use bigneon_db::models::*;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[actix_rt::test]
    async fn index_org_member() {
        base::event_report_subscribers::index(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn index_admin() {
        base::event_report_subscribers::index(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn index_user() {
        base::event_report_subscribers::index(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn index_org_owner() {
        base::event_report_subscribers::index(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn index_door_person() {
        base::event_report_subscribers::index(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn index_promoter() {
        base::event_report_subscribers::index(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn index_promoter_read_only() {
        base::event_report_subscribers::index(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn index_org_admin() {
        base::event_report_subscribers::index(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn index_box_office() {
        base::event_report_subscribers::index(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::event_report_subscribers::create(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::event_report_subscribers::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::event_report_subscribers::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::event_report_subscribers::create(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::event_report_subscribers::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::event_report_subscribers::create(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::event_report_subscribers::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::event_report_subscribers::create(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::event_report_subscribers::create(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[actix_rt::test]
    async fn destroy_org_member() {
        base::event_report_subscribers::destroy(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn destroy_admin() {
        base::event_report_subscribers::destroy(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn destroy_user() {
        base::event_report_subscribers::destroy(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_owner() {
        base::event_report_subscribers::destroy(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn destroy_door_person() {
        base::event_report_subscribers::destroy(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter() {
        base::event_report_subscribers::destroy(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter_read_only() {
        base::event_report_subscribers::destroy(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_admin() {
        base::event_report_subscribers::destroy(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn destroy_box_office() {
        base::event_report_subscribers::destroy(Roles::OrgBoxOffice, false).await;
    }
}
