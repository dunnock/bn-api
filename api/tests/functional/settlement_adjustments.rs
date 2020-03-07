use bigneon_db::models::*;
use crate::functional::base;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn index_org_member() {
        base::settlement_adjustments::index(Roles::OrgMember, false);
    }
    #[test]
    fn index_admin() {
        base::settlement_adjustments::index(Roles::Admin, true);
    }
    #[test]
    fn index_user() {
        base::settlement_adjustments::index(Roles::User, false);
    }
    #[test]
    fn index_org_owner() {
        base::settlement_adjustments::index(Roles::OrgOwner, false);
    }
    #[test]
    fn index_door_person() {
        base::settlement_adjustments::index(Roles::DoorPerson, false);
    }
    #[test]
    fn index_promoter() {
        base::settlement_adjustments::index(Roles::Promoter, false);
    }
    #[test]
    fn index_promoter_read_only() {
        base::settlement_adjustments::index(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn index_org_admin() {
        base::settlement_adjustments::index(Roles::OrgAdmin, false);
    }
    #[test]
    fn index_box_office() {
        base::settlement_adjustments::index(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::settlement_adjustments::create(Roles::OrgMember, false);
    }
    #[test]
    fn create_admin() {
        base::settlement_adjustments::create(Roles::Admin, true);
    }
    #[test]
    fn create_user() {
        base::settlement_adjustments::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::settlement_adjustments::create(Roles::OrgOwner, false);
    }
    #[test]
    fn create_door_person() {
        base::settlement_adjustments::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::settlement_adjustments::create(Roles::Promoter, false);
    }
    #[test]
    fn create_promoter_read_only() {
        base::settlement_adjustments::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::settlement_adjustments::create(Roles::OrgAdmin, false);
    }
    #[test]
    fn create_box_office() {
        base::settlement_adjustments::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[test]
    fn destroy_org_member() {
        base::settlement_adjustments::destroy(Roles::OrgMember, false);
    }
    #[test]
    fn destroy_admin() {
        base::settlement_adjustments::destroy(Roles::Admin, true);
    }
    #[test]
    fn destroy_user() {
        base::settlement_adjustments::destroy(Roles::User, false);
    }
    #[test]
    fn destroy_org_owner() {
        base::settlement_adjustments::destroy(Roles::OrgOwner, false);
    }
    #[test]
    fn destroy_door_person() {
        base::settlement_adjustments::destroy(Roles::DoorPerson, false);
    }
    #[test]
    fn destroy_promoter() {
        base::settlement_adjustments::destroy(Roles::Promoter, false);
    }
    #[test]
    fn destroy_promoter_read_only() {
        base::settlement_adjustments::destroy(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn destroy_org_admin() {
        base::settlement_adjustments::destroy(Roles::OrgAdmin, false);
    }
    #[test]
    fn destroy_box_office() {
        base::settlement_adjustments::destroy(Roles::OrgBoxOffice, false);
    }
}
