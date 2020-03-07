use bigneon_db::models::*;
use crate::functional::base;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn index_org_member() {
        base::event_report_subscribers::index(Roles::OrgMember, true);
    }
    #[test]
    fn index_admin() {
        base::event_report_subscribers::index(Roles::Admin, true);
    }
    #[test]
    fn index_user() {
        base::event_report_subscribers::index(Roles::User, false);
    }
    #[test]
    fn index_org_owner() {
        base::event_report_subscribers::index(Roles::OrgOwner, true);
    }
    #[test]
    fn index_door_person() {
        base::event_report_subscribers::index(Roles::DoorPerson, false);
    }
    #[test]
    fn index_promoter() {
        base::event_report_subscribers::index(Roles::Promoter, false);
    }
    #[test]
    fn index_promoter_read_only() {
        base::event_report_subscribers::index(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn index_org_admin() {
        base::event_report_subscribers::index(Roles::OrgAdmin, true);
    }
    #[test]
    fn index_box_office() {
        base::event_report_subscribers::index(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::event_report_subscribers::create(Roles::OrgMember, true);
    }
    #[test]
    fn create_admin() {
        base::event_report_subscribers::create(Roles::Admin, true);
    }
    #[test]
    fn create_user() {
        base::event_report_subscribers::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::event_report_subscribers::create(Roles::OrgOwner, true);
    }
    #[test]
    fn create_door_person() {
        base::event_report_subscribers::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::event_report_subscribers::create(Roles::Promoter, false);
    }
    #[test]
    fn create_promoter_read_only() {
        base::event_report_subscribers::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::event_report_subscribers::create(Roles::OrgAdmin, true);
    }
    #[test]
    fn create_box_office() {
        base::event_report_subscribers::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[test]
    fn destroy_org_member() {
        base::event_report_subscribers::destroy(Roles::OrgMember, true);
    }
    #[test]
    fn destroy_admin() {
        base::event_report_subscribers::destroy(Roles::Admin, true);
    }
    #[test]
    fn destroy_user() {
        base::event_report_subscribers::destroy(Roles::User, false);
    }
    #[test]
    fn destroy_org_owner() {
        base::event_report_subscribers::destroy(Roles::OrgOwner, true);
    }
    #[test]
    fn destroy_door_person() {
        base::event_report_subscribers::destroy(Roles::DoorPerson, false);
    }
    #[test]
    fn destroy_promoter() {
        base::event_report_subscribers::destroy(Roles::Promoter, false);
    }
    #[test]
    fn destroy_promoter_read_only() {
        base::event_report_subscribers::destroy(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn destroy_org_admin() {
        base::event_report_subscribers::destroy(Roles::OrgAdmin, true);
    }
    #[test]
    fn destroy_box_office() {
        base::event_report_subscribers::destroy(Roles::OrgBoxOffice, false);
    }
}
