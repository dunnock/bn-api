use crate::functional::base::reports_admin;
use bigneon_db::prelude::*;

#[cfg(test)]
mod domain_transaction_detail_report_tests {
    use super::*;
    #[actix_rt::test]
    async fn domain_transaction_detail_report_org_member() {
        reports_admin::domain_transaction_detail_report(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn domain_transaction_detail_report_admin() {
        reports_admin::domain_transaction_detail_report(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn domain_transaction_detail_report_user() {
        reports_admin::domain_transaction_detail_report(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn domain_transaction_detail_report_org_owner() {
        reports_admin::domain_transaction_detail_report(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn domain_transaction_detail_report_door_person() {
        reports_admin::domain_transaction_detail_report(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn domain_transaction_detail_report_promoter() {
        reports_admin::domain_transaction_detail_report(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn domain_transaction_detail_report_promoter_read_only() {
        reports_admin::domain_transaction_detail_report(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn domain_transaction_detail_report_org_admin() {
        reports_admin::domain_transaction_detail_report(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn domain_transaction_detail_report_box_office() {
        reports_admin::domain_transaction_detail_report(Roles::OrgBoxOffice, false).await;
    }
}
