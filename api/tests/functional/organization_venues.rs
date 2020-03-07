use bigneon_db::models::*;
use crate::functional::base;

#[cfg(test)]
mod show_tests {
    use super::*;
    #[test]
    fn show_org_member() {
        base::organization_venues::show(Roles::OrgMember, false);
    }
    #[test]
    fn show_admin() {
        base::organization_venues::show(Roles::Admin, true);
    }
    #[test]
    fn show_super() {
        base::organization_venues::show(Roles::Super, true);
    }
    #[test]
    fn show_user() {
        base::organization_venues::show(Roles::User, false);
    }
    #[test]
    fn show_org_owner() {
        base::organization_venues::show(Roles::OrgOwner, false);
    }
    #[test]
    fn show_door_person() {
        base::organization_venues::show(Roles::DoorPerson, false);
    }
    #[test]
    fn show_promoter() {
        base::organization_venues::show(Roles::Promoter, false);
    }
    #[test]
    fn show_promoter_read_only() {
        base::organization_venues::show(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn show_org_admin() {
        base::organization_venues::show(Roles::OrgAdmin, false);
    }
    #[test]
    fn show_box_office() {
        base::organization_venues::show(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::organization_venues::create(Roles::OrgMember, false);
    }
    #[test]
    fn create_admin() {
        base::organization_venues::create(Roles::Admin, true);
    }
    #[test]
    fn create_super() {
        base::organization_venues::create(Roles::Super, true);
    }
    #[test]
    fn create_user() {
        base::organization_venues::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::organization_venues::create(Roles::OrgOwner, false);
    }
    #[test]
    fn create_door_person() {
        base::organization_venues::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::organization_venues::create(Roles::Promoter, false);
    }
    #[test]
    fn create_promoter_read_only() {
        base::organization_venues::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::organization_venues::create(Roles::OrgAdmin, false);
    }
    #[test]
    fn create_box_office() {
        base::organization_venues::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod venues_index_tests {
    use super::*;
    #[test]
    fn venues_venues_index_org_member() {
        base::organization_venues::venues_index(Roles::OrgMember, false);
    }
    #[test]
    fn venues_index_admin() {
        base::organization_venues::venues_index(Roles::Admin, true);
    }
    #[test]
    fn venues_index_super() {
        base::organization_venues::venues_index(Roles::Super, true);
    }
    #[test]
    fn venues_index_user() {
        base::organization_venues::venues_index(Roles::User, false);
    }
    #[test]
    fn venues_index_org_owner() {
        base::organization_venues::venues_index(Roles::OrgOwner, false);
    }
    #[test]
    fn venues_index_door_person() {
        base::organization_venues::venues_index(Roles::DoorPerson, false);
    }
    #[test]
    fn venues_index_promoter() {
        base::organization_venues::venues_index(Roles::Promoter, false);
    }
    #[test]
    fn venues_index_promoter_read_only() {
        base::organization_venues::venues_index(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn venues_index_org_admin() {
        base::organization_venues::venues_index(Roles::OrgAdmin, false);
    }
    #[test]
    fn venues_index_box_office() {
        base::organization_venues::venues_index(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod index_organization_id_tests {
    use super::*;
    #[test]
    fn organizations_index_org_member() {
        base::organization_venues::organizations_index(Roles::OrgMember, false);
    }
    #[test]
    fn organizations_index_admin() {
        base::organization_venues::organizations_index(Roles::Admin, true);
    }
    #[test]
    fn organizations_index_super() {
        base::organization_venues::organizations_index(Roles::Super, true);
    }
    #[test]
    fn organizations_index_user() {
        base::organization_venues::organizations_index(Roles::User, false);
    }
    #[test]
    fn organizations_index_org_owner() {
        base::organization_venues::organizations_index(Roles::OrgOwner, false);
    }
    #[test]
    fn organizations_index_door_person() {
        base::organization_venues::organizations_index(Roles::DoorPerson, false);
    }
    #[test]
    fn organizations_index_promoter() {
        base::organization_venues::organizations_index(Roles::Promoter, false);
    }
    #[test]
    fn organizations_index_promoter_read_only() {
        base::organization_venues::organizations_index(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn organizations_index_org_admin() {
        base::organization_venues::organizations_index(Roles::OrgAdmin, false);
    }
    #[test]
    fn organizations_index_box_office() {
        base::organization_venues::organizations_index(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[test]
    fn destroy_org_member() {
        base::organization_venues::destroy(Roles::OrgMember, 1, false);
    }
    #[test]
    fn destroy_admin() {
        base::organization_venues::destroy(Roles::Admin, 1, true);
    }
    #[test]
    fn destroy_super() {
        base::organization_venues::destroy(Roles::Super, 1, true);
    }
    #[test]
    fn destroy_user() {
        base::organization_venues::destroy(Roles::User, 1, false);
    }
    #[test]
    fn destroy_org_owner() {
        base::organization_venues::destroy(Roles::OrgOwner, 1, false);
    }
    #[test]
    fn destroy_door_person() {
        base::organization_venues::destroy(Roles::DoorPerson, 1, false);
    }
    #[test]
    fn destroy_promoter() {
        base::organization_venues::destroy(Roles::Promoter, 1, false);
    }
    #[test]
    fn destroy_promoter_read_only() {
        base::organization_venues::destroy(Roles::PromoterReadOnly, 1, false);
    }
    #[test]
    fn destroy_org_admin() {
        base::organization_venues::destroy(Roles::OrgAdmin, 1, false);
    }
    #[test]
    fn destroy_box_office() {
        base::organization_venues::destroy(Roles::OrgBoxOffice, 1, false);
    }
}

#[cfg(test)]
mod destroy_last_venue_link_tests {
    use super::*;
    #[test]
    fn destroy_last_venue_link_org_member() {
        base::organization_venues::destroy(Roles::OrgMember, 0, false);
    }
    #[test]
    fn destroy_last_venue_link_admin() {
        base::organization_venues::destroy(Roles::Admin, 0, true);
    }
    #[test]
    fn destroy_last_venue_link_super() {
        base::organization_venues::destroy(Roles::Super, 0, true);
    }
    #[test]
    fn destroy_last_venue_link_user() {
        base::organization_venues::destroy(Roles::User, 0, false);
    }
    #[test]
    fn destroy_last_venue_link_org_owner() {
        base::organization_venues::destroy(Roles::OrgOwner, 0, false);
    }
    #[test]
    fn destroy_last_venue_link_door_person() {
        base::organization_venues::destroy(Roles::DoorPerson, 0, false);
    }
    #[test]
    fn destroy_last_venue_link_promoter() {
        base::organization_venues::destroy(Roles::Promoter, 0, false);
    }
    #[test]
    fn destroy_last_venue_link_promoter_read_only() {
        base::organization_venues::destroy(Roles::PromoterReadOnly, 0, false);
    }
    #[test]
    fn destroy_last_venue_link_org_admin() {
        base::organization_venues::destroy(Roles::OrgAdmin, 0, false);
    }
    #[test]
    fn destroy_last_venue_link_box_office() {
        base::organization_venues::destroy(Roles::OrgBoxOffice, 0, false);
    }
}
