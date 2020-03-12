use bigneon_db::prelude::*;
use crate::functional::base::reports_admin;

#[cfg(test)]
mod domain_transaction_detail_report_tests {
    use super::*;
    #[test]
    fn domain_transaction_detail_report_org_member() {
        reports_admin::domain_transaction_detail_report(Roles::OrgMember, false);
    }
    #[test]
    fn domain_transaction_detail_report_admin() {
        reports_admin::domain_transaction_detail_report(Roles::Admin, true);
    }
    #[test]
    fn domain_transaction_detail_report_user() {
        reports_admin::domain_transaction_detail_report(Roles::User, false);
    }
    #[test]
    fn domain_transaction_detail_report_org_owner() {
        reports_admin::domain_transaction_detail_report(Roles::OrgOwner, false);
    }
    #[test]
    fn domain_transaction_detail_report_door_person() {
        reports_admin::domain_transaction_detail_report(Roles::DoorPerson, false);
    }
    #[test]
    fn domain_transaction_detail_report_promoter() {
        reports_admin::domain_transaction_detail_report(Roles::Promoter, false);
    }
    #[test]
    fn domain_transaction_detail_report_promoter_read_only() {
        reports_admin::domain_transaction_detail_report(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn domain_transaction_detail_report_org_admin() {
        reports_admin::domain_transaction_detail_report(Roles::OrgAdmin, false);
    }
    #[test]
    fn domain_transaction_detail_report_box_office() {
        reports_admin::domain_transaction_detail_report(Roles::OrgBoxOffice, false);
    }
}
