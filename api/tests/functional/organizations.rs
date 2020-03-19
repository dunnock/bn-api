use crate::functional::base::organizations;
use db::models::Roles;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[actix_rt::test]
    async fn index_org_member() {
        organizations::index(Roles::OrgMember).await;
    }
    #[actix_rt::test]
    async fn index_admin() {
        organizations::index(Roles::Admin).await;
    }
    #[actix_rt::test]
    async fn index_user() {
        organizations::index(Roles::User).await;
    }
    #[actix_rt::test]
    async fn index_org_owner() {
        organizations::index(Roles::OrgOwner).await;
    }
    #[actix_rt::test]
    async fn index_door_person() {
        organizations::index(Roles::DoorPerson).await;
    }
    #[actix_rt::test]
    async fn index_promoter() {
        organizations::index(Roles::Promoter).await;
    }
    #[actix_rt::test]
    async fn index_promoter_read_only() {
        organizations::index(Roles::PromoterReadOnly).await;
    }
    #[actix_rt::test]
    async fn index_org_admin() {
        organizations::index(Roles::OrgAdmin).await;
    }
    #[actix_rt::test]
    async fn index_box_office() {
        organizations::index(Roles::OrgBoxOffice).await;
    }
}

#[cfg(test)]
mod show_tests {
    use super::*;
    #[actix_rt::test]
    async fn show_org_member() {
        organizations::show(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn show_admin() {
        organizations::show(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn show_user() {
        organizations::show(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn show_org_owner() {
        organizations::show(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn show_door_person() {
        organizations::show(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn show_promoter() {
        organizations::show(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn show_promoter_read_only() {
        organizations::show(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn show_org_admin() {
        organizations::show(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn show_box_office() {
        organizations::show(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod index_for_all_orgs_tests {
    use super::*;
    #[actix_rt::test]
    async fn index_for_all_orgs_org_member() {
        organizations::index_for_all_orgs(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn index_for_all_orgs_admin() {
        organizations::index_for_all_orgs(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn index_for_all_orgs_user() {
        organizations::index_for_all_orgs(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn index_for_all_orgs_org_owner() {
        organizations::index_for_all_orgs(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn index_for_all_orgs_door_person() {
        organizations::index_for_all_orgs(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn index_for_all_orgs_promoter() {
        organizations::index_for_all_orgs(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn index_for_all_orgs_promoter_read_only() {
        organizations::index_for_all_orgs(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn index_for_all_orgs_org_admin() {
        organizations::index_for_all_orgs(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn index_for_all_orgs_box_office() {
        organizations::index_for_all_orgs(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        organizations::create(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        organizations::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        organizations::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        organizations::create(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        organizations::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        organizations::create(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        organizations::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        organizations::create(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        organizations::create(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod remove_user_tests {
    use super::*;
    #[actix_rt::test]
    async fn remove_user_org_member() {
        organizations::remove_user(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn remove_user_admin() {
        organizations::remove_user(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn remove_user_user() {
        organizations::remove_user(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn remove_user_org_owner() {
        organizations::remove_user(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn remove_user_door_person() {
        organizations::remove_user(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn remove_user_promoter() {
        organizations::remove_user(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn remove_user_promoter_read_only() {
        organizations::remove_user(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn remove_user_org_admin() {
        organizations::remove_user(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn remove_user_box_office() {
        organizations::remove_user(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod add_user_tests {
    use super::*;
    #[actix_rt::test]
    async fn add_user_org_member() {
        organizations::add_user(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn add_user_admin() {
        organizations::add_user(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn add_user_user() {
        organizations::add_user(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn add_user_org_owner() {
        organizations::add_user(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn add_user_door_person() {
        organizations::add_user(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn add_user_promoter() {
        organizations::add_user(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn add_user_promoter_read_only() {
        organizations::add_user(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn add_user_org_admin() {
        organizations::add_user(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn add_user_box_office() {
        organizations::add_user(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod add_artist_tests {
    use super::*;
    #[actix_rt::test]
    async fn add_artist_org_member() {
        organizations::add_artist(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn add_artist_admin() {
        organizations::add_artist(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn add_artist_user() {
        organizations::add_artist(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn add_artist_org_owner() {
        organizations::add_artist(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn add_artist_door_person() {
        organizations::add_artist(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn add_artist_promoter() {
        organizations::add_artist(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn add_artist_promoter_read_only() {
        organizations::add_artist(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn add_artist_org_admin() {
        organizations::add_artist(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn add_artist_box_office() {
        organizations::add_artist(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_org_member() {
        organizations::update(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn update_admin() {
        organizations::update(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_user() {
        organizations::update(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn update_org_owner() {
        organizations::update(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn update_door_person() {
        organizations::update(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter() {
        organizations::update(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter_read_only() {
        organizations::update(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn update_org_admin() {
        organizations::update(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn update_box_office() {
        organizations::update(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_tests_with_settlement_type {
    use super::*;
    #[actix_rt::test]
    async fn update_org_member() {
        organizations::update_restricted_field("settlement_type", Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn update_super() {
        organizations::update_restricted_field("settlement_type", Roles::Super, true).await;
    }
    #[actix_rt::test]
    async fn update_admin() {
        organizations::update_restricted_field("settlement_type", Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_user() {
        organizations::update_restricted_field("settlement_type", Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn update_org_owner() {
        organizations::update_restricted_field("settlement_type", Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn update_door_person() {
        organizations::update_restricted_field("settlement_type", Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter() {
        organizations::update_restricted_field("settlement_type", Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter_read_only() {
        organizations::update_restricted_field("settlement_type", Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn update_org_admin() {
        organizations::update_restricted_field("settlement_type", Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn update_box_office() {
        organizations::update_restricted_field("settlement_type", Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_tests_with_max_tickets_per_ticket_type {
    use super::*;
    #[actix_rt::test]
    async fn update_org_member() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn update_admin() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_user() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn update_org_owner() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn update_door_person() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter_read_only() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn update_org_admin() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn update_box_office() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod list_organization_members_tests {
    use super::*;
    #[actix_rt::test]
    async fn list_organization_members_org_member() {
        organizations::list_organization_members(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn list_organization_members_admin() {
        organizations::list_organization_members(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn list_organization_members_user() {
        organizations::list_organization_members(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn list_organization_members_org_owner() {
        organizations::list_organization_members(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn list_organization_members_door_person() {
        organizations::list_organization_members(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn list_organization_members_promoter() {
        organizations::list_organization_members(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn list_organization_members_promoter_read_only() {
        organizations::list_organization_members(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn list_organization_members_org_admin() {
        organizations::list_organization_members(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn list_organization_members_box_office() {
        organizations::list_organization_members(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod show_fee_schedule_tests {
    use super::*;
    #[actix_rt::test]
    async fn show_fee_schedule_org_member() {
        organizations::show_fee_schedule(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn show_fee_schedule_admin() {
        organizations::show_fee_schedule(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn show_fee_schedule_user() {
        organizations::show_fee_schedule(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn show_fee_schedule_org_owner() {
        organizations::show_fee_schedule(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn show_fee_schedule_door_person() {
        organizations::show_fee_schedule(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn show_fee_schedule_promoter() {
        organizations::show_fee_schedule(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn show_fee_schedule_promoter_read_only() {
        organizations::show_fee_schedule(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn show_fee_schedule_org_admin() {
        organizations::show_fee_schedule(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn show_fee_schedule_box_office() {
        organizations::show_fee_schedule(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod add_fee_schedule_tests {
    use super::*;
    #[actix_rt::test]
    async fn add_fee_schedule_org_member() {
        organizations::add_fee_schedule(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn add_fee_schedule_admin() {
        organizations::add_fee_schedule(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn add_fee_schedule_user() {
        organizations::add_fee_schedule(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn add_fee_schedule_org_owner() {
        organizations::add_fee_schedule(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn add_fee_schedule_door_person() {
        organizations::add_fee_schedule(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn add_fee_schedule_promoter_read_only() {
        organizations::add_fee_schedule(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn add_fee_schedule_org_admin() {
        organizations::add_fee_schedule(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn add_fee_schedule_box_office() {
        organizations::add_fee_schedule(Roles::OrgBoxOffice, false).await;
    }
}
