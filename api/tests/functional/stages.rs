use bigneon_db::models::*;
use crate::functional::base;

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::stages::create(Roles::OrgMember, false);
    }
    #[test]
    fn create_admin() {
        base::stages::create(Roles::Admin, true);
    }
    #[test]
    fn create_user() {
        base::stages::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::stages::create(Roles::OrgOwner, false);
    }
    #[test]
    fn create_door_person() {
        base::stages::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::stages::create(Roles::Promoter, false);
    }
    #[test]
    fn create_promoter_read_only() {
        base::stages::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::stages::create(Roles::OrgAdmin, false);
    }
    #[test]
    fn create_box_office() {
        base::stages::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        base::stages::update(Roles::OrgMember, false);
    }
    #[test]
    fn update_admin() {
        base::stages::update(Roles::Admin, true);
    }
    #[test]
    fn update_user() {
        base::stages::update(Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        base::stages::update(Roles::OrgOwner, false);
    }
    #[test]
    fn update_door_person() {
        base::stages::update(Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        base::stages::update(Roles::Promoter, false);
    }
    #[test]
    fn update_promoter_read_only() {
        base::stages::update(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        base::stages::update(Roles::OrgAdmin, false);
    }
    #[test]
    fn update_box_office() {
        base::stages::update(Roles::OrgBoxOffice, false);
    }
}
