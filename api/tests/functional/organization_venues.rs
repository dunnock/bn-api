use bigneon_db::models::*;
use functional::base;

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
mod index_venue_id_tests {
    use super::*;
    #[test]
    fn index_venue_id_org_member() {
        base::organization_venues::index(Roles::OrgMember, false, false);
    }
    #[test]
    fn index_venue_id_admin() {
        base::organization_venues::index(Roles::Admin, false, true);
    }
    #[test]
    fn index_venue_id_super() {
        base::organization_venues::index(Roles::Super, false, true);
    }
    #[test]
    fn index_venue_id_user() {
        base::organization_venues::index(Roles::User, false, false);
    }
    #[test]
    fn index_venue_id_org_owner() {
        base::organization_venues::index(Roles::OrgOwner, false, false);
    }
    #[test]
    fn index_venue_id_door_person() {
        base::organization_venues::index(Roles::DoorPerson, false, false);
    }
    #[test]
    fn index_venue_id_promoter() {
        base::organization_venues::index(Roles::Promoter, false, false);
    }
    #[test]
    fn index_venue_id_promoter_read_only() {
        base::organization_venues::index(Roles::PromoterReadOnly, false, false);
    }
    #[test]
    fn index_venue_id_org_admin() {
        base::organization_venues::index(Roles::OrgAdmin, false, false);
    }
    #[test]
    fn index_venue_id_box_office() {
        base::organization_venues::index(Roles::OrgBoxOffice, false, false);
    }
}

#[cfg(test)]
mod index_organization_id_tests {
    use super::*;
    #[test]
    fn index_organization_id_org_member() {
        base::organization_venues::index(Roles::OrgMember, true, false);
    }
    #[test]
    fn index_organization_id_admin() {
        base::organization_venues::index(Roles::Admin, true, true);
    }
    #[test]
    fn index_organization_id_super() {
        base::organization_venues::index(Roles::Super, true, true);
    }
    #[test]
    fn index_organization_id_user() {
        base::organization_venues::index(Roles::User, true, false);
    }
    #[test]
    fn index_organization_id_org_owner() {
        base::organization_venues::index(Roles::OrgOwner, true, false);
    }
    #[test]
    fn index_organization_id_door_person() {
        base::organization_venues::index(Roles::DoorPerson, true, false);
    }
    #[test]
    fn index_organization_id_promoter() {
        base::organization_venues::index(Roles::Promoter, true, false);
    }
    #[test]
    fn index_organization_id_promoter_read_only() {
        base::organization_venues::index(Roles::PromoterReadOnly, true, false);
    }
    #[test]
    fn index_organization_id_org_admin() {
        base::organization_venues::index(Roles::OrgAdmin, true, false);
    }
    #[test]
    fn index_organization_id_box_office() {
        base::organization_venues::index(Roles::OrgBoxOffice, true, false);
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
