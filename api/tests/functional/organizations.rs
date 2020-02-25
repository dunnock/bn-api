use bigneon_db::models::Roles;
use functional::base::organizations;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn index_org_member() {
        organizations::index(Roles::OrgMember);
    }
    #[test]
    fn index_admin() {
        organizations::index(Roles::Admin);
    }
    #[test]
    fn index_user() {
        organizations::index(Roles::User);
    }
    #[test]
    fn index_org_owner() {
        organizations::index(Roles::OrgOwner);
    }
    #[test]
    fn index_door_person() {
        organizations::index(Roles::DoorPerson);
    }
    #[test]
    fn index_promoter() {
        organizations::index(Roles::Promoter);
    }
    #[test]
    fn index_promoter_read_only() {
        organizations::index(Roles::PromoterReadOnly);
    }
    #[test]
    fn index_org_admin() {
        organizations::index(Roles::OrgAdmin);
    }
    #[test]
    fn index_box_office() {
        organizations::index(Roles::OrgBoxOffice);
    }
}

#[cfg(test)]
mod show_tests {
    use super::*;
    #[test]
    fn show_org_member() {
        organizations::show(Roles::OrgMember, true);
    }
    #[test]
    fn show_admin() {
        organizations::show(Roles::Admin, true);
    }
    #[test]
    fn show_user() {
        organizations::show(Roles::User, false);
    }
    #[test]
    fn show_org_owner() {
        organizations::show(Roles::OrgOwner, true);
    }
    #[test]
    fn show_door_person() {
        organizations::show(Roles::DoorPerson, false);
    }
    #[test]
    fn show_promoter() {
        organizations::show(Roles::Promoter, false);
    }
    #[test]
    fn show_promoter_read_only() {
        organizations::show(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn show_org_admin() {
        organizations::show(Roles::OrgAdmin, true);
    }
    #[test]
    fn show_box_office() {
        organizations::show(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod index_for_all_orgs_tests {
    use super::*;
    #[test]
    fn index_for_all_orgs_org_member() {
        organizations::index_for_all_orgs(Roles::OrgMember, false);
    }
    #[test]
    fn index_for_all_orgs_admin() {
        organizations::index_for_all_orgs(Roles::Admin, true);
    }
    #[test]
    fn index_for_all_orgs_user() {
        organizations::index_for_all_orgs(Roles::User, false);
    }
    #[test]
    fn index_for_all_orgs_org_owner() {
        organizations::index_for_all_orgs(Roles::OrgOwner, false);
    }
    #[test]
    fn index_for_all_orgs_door_person() {
        organizations::index_for_all_orgs(Roles::DoorPerson, false);
    }
    #[test]
    fn index_for_all_orgs_promoter() {
        organizations::index_for_all_orgs(Roles::Promoter, false);
    }
    #[test]
    fn index_for_all_orgs_promoter_read_only() {
        organizations::index_for_all_orgs(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn index_for_all_orgs_org_admin() {
        organizations::index_for_all_orgs(Roles::OrgAdmin, false);
    }
    #[test]
    fn index_for_all_orgs_box_office() {
        organizations::index_for_all_orgs(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        organizations::create(Roles::OrgMember, false);
    }
    #[test]
    fn create_admin() {
        organizations::create(Roles::Admin, true);
    }
    #[test]
    fn create_user() {
        organizations::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        organizations::create(Roles::OrgOwner, false);
    }
    #[test]
    fn create_door_person() {
        organizations::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        organizations::create(Roles::Promoter, false);
    }
    #[test]
    fn create_promoter_read_only() {
        organizations::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        organizations::create(Roles::OrgAdmin, false);
    }
    #[test]
    fn create_box_office() {
        organizations::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod remove_user_tests {
    use super::*;
    #[test]
    fn remove_user_org_member() {
        organizations::remove_user(Roles::OrgMember, false);
    }
    #[test]
    fn remove_user_admin() {
        organizations::remove_user(Roles::Admin, true);
    }
    #[test]
    fn remove_user_user() {
        organizations::remove_user(Roles::User, false);
    }
    #[test]
    fn remove_user_org_owner() {
        organizations::remove_user(Roles::OrgOwner, true);
    }
    #[test]
    fn remove_user_door_person() {
        organizations::remove_user(Roles::DoorPerson, false);
    }
    #[test]
    fn remove_user_promoter() {
        organizations::remove_user(Roles::Promoter, false);
    }
    #[test]
    fn remove_user_promoter_read_only() {
        organizations::remove_user(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn remove_user_org_admin() {
        organizations::remove_user(Roles::OrgAdmin, true);
    }
    #[test]
    fn remove_user_box_office() {
        organizations::remove_user(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod add_user_tests {
    use super::*;
    #[test]
    fn add_user_org_member() {
        organizations::add_user(Roles::OrgMember, false);
    }
    #[test]
    fn add_user_admin() {
        organizations::add_user(Roles::Admin, true);
    }
    #[test]
    fn add_user_user() {
        organizations::add_user(Roles::User, false);
    }
    #[test]
    fn add_user_org_owner() {
        organizations::add_user(Roles::OrgOwner, true);
    }
    #[test]
    fn add_user_door_person() {
        organizations::add_user(Roles::DoorPerson, false);
    }
    #[test]
    fn add_user_promoter() {
        organizations::add_user(Roles::Promoter, false);
    }
    #[test]
    fn add_user_promoter_read_only() {
        organizations::add_user(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn add_user_org_admin() {
        organizations::add_user(Roles::OrgAdmin, true);
    }
    #[test]
    fn add_user_box_office() {
        organizations::add_user(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod add_artist_tests {
    use super::*;
    #[test]
    fn add_artist_org_member() {
        organizations::add_artist(Roles::OrgMember, false);
    }
    #[test]
    fn add_artist_admin() {
        organizations::add_artist(Roles::Admin, true);
    }
    #[test]
    fn add_artist_user() {
        organizations::add_artist(Roles::User, false);
    }
    #[test]
    fn add_artist_org_owner() {
        organizations::add_artist(Roles::OrgOwner, true);
    }
    #[test]
    fn add_artist_door_person() {
        organizations::add_artist(Roles::DoorPerson, false);
    }
    #[test]
    fn add_artist_promoter() {
        organizations::add_artist(Roles::Promoter, false);
    }
    #[test]
    fn add_artist_promoter_read_only() {
        organizations::add_artist(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn add_artist_org_admin() {
        organizations::add_artist(Roles::OrgAdmin, true);
    }
    #[test]
    fn add_artist_box_office() {
        organizations::add_artist(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        organizations::update(Roles::OrgMember, false);
    }
    #[test]
    fn update_admin() {
        organizations::update(Roles::Admin, true);
    }
    #[test]
    fn update_user() {
        organizations::update(Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        organizations::update(Roles::OrgOwner, true);
    }
    #[test]
    fn update_door_person() {
        organizations::update(Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        organizations::update(Roles::Promoter, false);
    }
    #[test]
    fn update_promoter_read_only() {
        organizations::update(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        organizations::update(Roles::OrgAdmin, true);
    }
    #[test]
    fn update_box_office() {
        organizations::update(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests_with_settlement_type {
    use super::*;
    #[test]
    fn update_org_member() {
        organizations::update_restricted_field("settlement_type", Roles::OrgMember, false);
    }
    #[test]
    fn update_super() {
        organizations::update_restricted_field("settlement_type", Roles::Super, true);
    }
    #[test]
    fn update_admin() {
        organizations::update_restricted_field("settlement_type", Roles::Admin, true);
    }
    #[test]
    fn update_user() {
        organizations::update_restricted_field("settlement_type", Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        organizations::update_restricted_field("settlement_type", Roles::OrgOwner, false);
    }
    #[test]
    fn update_door_person() {
        organizations::update_restricted_field("settlement_type", Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        organizations::update_restricted_field("settlement_type", Roles::Promoter, false);
    }
    #[test]
    fn update_promoter_read_only() {
        organizations::update_restricted_field("settlement_type", Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        organizations::update_restricted_field("settlement_type", Roles::OrgAdmin, false);
    }
    #[test]
    fn update_box_office() {
        organizations::update_restricted_field("settlement_type", Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests_with_max_tickets_per_ticket_type {
    use super::*;
    #[test]
    fn update_org_member() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::OrgMember, false);
    }
    #[test]
    fn update_admin() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::Admin, true);
    }
    #[test]
    fn update_user() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::OrgOwner, false);
    }
    #[test]
    fn update_door_person() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::Promoter, false);
    }
    #[test]
    fn update_promoter_read_only() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::OrgAdmin, false);
    }
    #[test]
    fn update_box_office() {
        organizations::update_restricted_field("max_instances_per_ticket_type", Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod list_organization_members_tests {
    use super::*;
    #[test]
    fn list_organization_members_org_member() {
        organizations::list_organization_members(Roles::OrgMember, true);
    }
    #[test]
    fn list_organization_members_admin() {
        organizations::list_organization_members(Roles::Admin, true);
    }
    #[test]
    fn list_organization_members_user() {
        organizations::list_organization_members(Roles::User, false);
    }
    #[test]
    fn list_organization_members_org_owner() {
        organizations::list_organization_members(Roles::OrgOwner, true);
    }
    #[test]
    fn list_organization_members_door_person() {
        organizations::list_organization_members(Roles::DoorPerson, false);
    }
    #[test]
    fn list_organization_members_promoter() {
        organizations::list_organization_members(Roles::Promoter, false);
    }
    #[test]
    fn list_organization_members_promoter_read_only() {
        organizations::list_organization_members(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn list_organization_members_org_admin() {
        organizations::list_organization_members(Roles::OrgAdmin, true);
    }
    #[test]
    fn list_organization_members_box_office() {
        organizations::list_organization_members(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod show_fee_schedule_tests {
    use super::*;
    #[test]
    fn show_fee_schedule_org_member() {
        organizations::show_fee_schedule(Roles::OrgMember, false);
    }
    #[test]
    fn show_fee_schedule_admin() {
        organizations::show_fee_schedule(Roles::Admin, true);
    }
    #[test]
    fn show_fee_schedule_user() {
        organizations::show_fee_schedule(Roles::User, false);
    }
    #[test]
    fn show_fee_schedule_org_owner() {
        organizations::show_fee_schedule(Roles::OrgOwner, true);
    }
    #[test]
    fn show_fee_schedule_door_person() {
        organizations::show_fee_schedule(Roles::DoorPerson, false);
    }
    #[test]
    fn show_fee_schedule_promoter() {
        organizations::show_fee_schedule(Roles::Promoter, false);
    }
    #[test]
    fn show_fee_schedule_promoter_read_only() {
        organizations::show_fee_schedule(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn show_fee_schedule_org_admin() {
        organizations::show_fee_schedule(Roles::OrgAdmin, true);
    }
    #[test]
    fn show_fee_schedule_box_office() {
        organizations::show_fee_schedule(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod add_fee_schedule_tests {
    use super::*;
    #[test]
    fn add_fee_schedule_org_member() {
        organizations::add_fee_schedule(Roles::OrgMember, false);
    }
    #[test]
    fn add_fee_schedule_admin() {
        organizations::add_fee_schedule(Roles::Admin, true);
    }
    #[test]
    fn add_fee_schedule_user() {
        organizations::add_fee_schedule(Roles::User, false);
    }
    #[test]
    fn add_fee_schedule_org_owner() {
        organizations::add_fee_schedule(Roles::OrgOwner, false);
    }
    #[test]
    fn add_fee_schedule_door_person() {
        organizations::add_fee_schedule(Roles::DoorPerson, false);
    }
    #[test]
    fn add_fee_schedule_promoter_read_only() {
        organizations::add_fee_schedule(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn add_fee_schedule_org_admin() {
        organizations::add_fee_schedule(Roles::OrgAdmin, false);
    }
    #[test]
    fn add_fee_schedule_box_office() {
        organizations::add_fee_schedule(Roles::OrgBoxOffice, false);
    }
}
