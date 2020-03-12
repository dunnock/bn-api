use crate::functional::base;
use bigneon_db::models::*;

#[cfg(test)]
mod show_tests {
    use super::*;
    #[actix_rt::test]
    async fn show_org_member() {
        base::organization_venues::show(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn show_admin() {
        base::organization_venues::show(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn show_super() {
        base::organization_venues::show(Roles::Super, true).await;
    }
    #[actix_rt::test]
    async fn show_user() {
        base::organization_venues::show(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn show_org_owner() {
        base::organization_venues::show(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn show_door_person() {
        base::organization_venues::show(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn show_promoter() {
        base::organization_venues::show(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn show_promoter_read_only() {
        base::organization_venues::show(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn show_org_admin() {
        base::organization_venues::show(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn show_box_office() {
        base::organization_venues::show(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::organization_venues::create(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::organization_venues::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_super() {
        base::organization_venues::create(Roles::Super, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::organization_venues::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::organization_venues::create(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::organization_venues::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::organization_venues::create(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::organization_venues::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::organization_venues::create(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::organization_venues::create(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod venues_index_tests {
    use super::*;
    #[actix_rt::test]
    async fn venues_venues_index_org_member() {
        base::organization_venues::venues_index(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn venues_index_admin() {
        base::organization_venues::venues_index(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn venues_index_super() {
        base::organization_venues::venues_index(Roles::Super, true).await;
    }
    #[actix_rt::test]
    async fn venues_index_user() {
        base::organization_venues::venues_index(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn venues_index_org_owner() {
        base::organization_venues::venues_index(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn venues_index_door_person() {
        base::organization_venues::venues_index(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn venues_index_promoter() {
        base::organization_venues::venues_index(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn venues_index_promoter_read_only() {
        base::organization_venues::venues_index(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn venues_index_org_admin() {
        base::organization_venues::venues_index(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn venues_index_box_office() {
        base::organization_venues::venues_index(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod index_organization_id_tests {
    use super::*;
    #[actix_rt::test]
    async fn organizations_index_org_member() {
        base::organization_venues::organizations_index(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn organizations_index_admin() {
        base::organization_venues::organizations_index(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn organizations_index_super() {
        base::organization_venues::organizations_index(Roles::Super, true).await;
    }
    #[actix_rt::test]
    async fn organizations_index_user() {
        base::organization_venues::organizations_index(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn organizations_index_org_owner() {
        base::organization_venues::organizations_index(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn organizations_index_door_person() {
        base::organization_venues::organizations_index(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn organizations_index_promoter() {
        base::organization_venues::organizations_index(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn organizations_index_promoter_read_only() {
        base::organization_venues::organizations_index(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn organizations_index_org_admin() {
        base::organization_venues::organizations_index(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn organizations_index_box_office() {
        base::organization_venues::organizations_index(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[actix_rt::test]
    async fn destroy_org_member() {
        base::organization_venues::destroy(Roles::OrgMember, 1, false).await;
    }
    #[actix_rt::test]
    async fn destroy_admin() {
        base::organization_venues::destroy(Roles::Admin, 1, true).await;
    }
    #[actix_rt::test]
    async fn destroy_super() {
        base::organization_venues::destroy(Roles::Super, 1, true).await;
    }
    #[actix_rt::test]
    async fn destroy_user() {
        base::organization_venues::destroy(Roles::User, 1, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_owner() {
        base::organization_venues::destroy(Roles::OrgOwner, 1, false).await;
    }
    #[actix_rt::test]
    async fn destroy_door_person() {
        base::organization_venues::destroy(Roles::DoorPerson, 1, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter() {
        base::organization_venues::destroy(Roles::Promoter, 1, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter_read_only() {
        base::organization_venues::destroy(Roles::PromoterReadOnly, 1, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_admin() {
        base::organization_venues::destroy(Roles::OrgAdmin, 1, false).await;
    }
    #[actix_rt::test]
    async fn destroy_box_office() {
        base::organization_venues::destroy(Roles::OrgBoxOffice, 1, false).await;
    }
}

#[cfg(test)]
mod destroy_last_venue_link_tests {
    use super::*;
    #[actix_rt::test]
    async fn destroy_last_venue_link_org_member() {
        base::organization_venues::destroy(Roles::OrgMember, 0, false).await;
    }
    #[actix_rt::test]
    async fn destroy_last_venue_link_admin() {
        base::organization_venues::destroy(Roles::Admin, 0, true).await;
    }
    #[actix_rt::test]
    async fn destroy_last_venue_link_super() {
        base::organization_venues::destroy(Roles::Super, 0, true).await;
    }
    #[actix_rt::test]
    async fn destroy_last_venue_link_user() {
        base::organization_venues::destroy(Roles::User, 0, false).await;
    }
    #[actix_rt::test]
    async fn destroy_last_venue_link_org_owner() {
        base::organization_venues::destroy(Roles::OrgOwner, 0, false).await;
    }
    #[actix_rt::test]
    async fn destroy_last_venue_link_door_person() {
        base::organization_venues::destroy(Roles::DoorPerson, 0, false).await;
    }
    #[actix_rt::test]
    async fn destroy_last_venue_link_promoter() {
        base::organization_venues::destroy(Roles::Promoter, 0, false).await;
    }
    #[actix_rt::test]
    async fn destroy_last_venue_link_promoter_read_only() {
        base::organization_venues::destroy(Roles::PromoterReadOnly, 0, false).await;
    }
    #[actix_rt::test]
    async fn destroy_last_venue_link_org_admin() {
        base::organization_venues::destroy(Roles::OrgAdmin, 0, false).await;
    }
    #[actix_rt::test]
    async fn destroy_last_venue_link_box_office() {
        base::organization_venues::destroy(Roles::OrgBoxOffice, 0, false).await;
    }
}
