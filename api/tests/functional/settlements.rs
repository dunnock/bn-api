use bigneon_db::models::*;
use crate::functional::base;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn index_org_member() {
        base::settlements::index(Roles::OrgMember, false);
    }
    #[test]
    fn index_admin() {
        base::settlements::index(Roles::Admin, true);
    }
    #[test]
    fn index_user() {
        base::settlements::index(Roles::User, false);
    }
    #[test]
    fn index_org_owner() {
        base::settlements::index(Roles::OrgOwner, true);
    }
    #[test]
    fn index_door_person() {
        base::settlements::index(Roles::DoorPerson, false);
    }
    #[test]
    fn index_promoter() {
        base::settlements::index(Roles::Promoter, false);
    }
    #[test]
    fn index_promoter_read_only() {
        base::settlements::index(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn index_org_admin() {
        base::settlements::index(Roles::OrgAdmin, true);
    }
    #[test]
    fn index_box_office() {
        base::settlements::index(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod show_tests {
    use super::*;
    #[test]
    fn show_org_member() {
        base::settlements::show(Roles::OrgMember, false);
    }
    #[test]
    fn show_admin() {
        base::settlements::show(Roles::Admin, true);
    }
    #[test]
    fn show_user() {
        base::settlements::show(Roles::User, false);
    }
    #[test]
    fn show_org_owner() {
        base::settlements::show(Roles::OrgOwner, true);
    }
    #[test]
    fn show_door_person() {
        base::settlements::show(Roles::DoorPerson, false);
    }
    #[test]
    fn show_promoter() {
        base::settlements::show(Roles::Promoter, false);
    }
    #[test]
    fn show_promoter_read_only() {
        base::settlements::show(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn show_org_admin() {
        base::settlements::show(Roles::OrgAdmin, true);
    }
    #[test]
    fn show_box_office() {
        base::settlements::show(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[test]
    fn destroy_org_member() {
        base::settlements::destroy(Roles::OrgMember, false);
    }
    #[test]
    fn destroy_admin() {
        base::settlements::destroy(Roles::Admin, true);
    }
    #[test]
    fn destroy_user() {
        base::settlements::destroy(Roles::User, false);
    }
    #[test]
    fn destroy_org_owner() {
        base::settlements::destroy(Roles::OrgOwner, false);
    }
    #[test]
    fn destroy_door_person() {
        base::settlements::destroy(Roles::DoorPerson, false);
    }
    #[test]
    fn destroy_promoter() {
        base::settlements::destroy(Roles::Promoter, false);
    }
    #[test]
    fn destroy_promoter_read_only() {
        base::settlements::destroy(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn destroy_org_admin() {
        base::settlements::destroy(Roles::OrgAdmin, false);
    }
    #[test]
    fn destroy_box_office() {
        base::settlements::destroy(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::settlements::create(Roles::OrgMember, false);
    }
    #[test]
    fn create_admin() {
        base::settlements::create(Roles::Admin, true);
    }
    #[test]
    fn create_user() {
        base::settlements::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::settlements::create(Roles::OrgOwner, false);
    }
    #[test]
    fn create_door_person() {
        base::settlements::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::settlements::create(Roles::Promoter, false);
    }
    #[test]
    fn create_promoter_read_only() {
        base::settlements::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::settlements::create(Roles::OrgAdmin, false);
    }
    #[test]
    fn create_box_office() {
        base::settlements::create(Roles::OrgBoxOffice, false);
    }
}
