use models::{AdditionalOrgMemberScopes, Roles};
use serde::Serialize;
use serde::Serializer;
use std::fmt;
use std::str::FromStr;
use utils::errors::EnumParseError;

#[derive(PartialEq, Debug, Copy, Clone, Eq, Ord, PartialOrd)]
pub enum Scopes {
    AnnouncementDelete,
    AnnouncementRead,
    AnnouncementWrite,
    AnnouncementEngagementWrite,
    ArtistWrite,
    BoxOfficeTicketRead,
    BoxOfficeTicketWrite,
    CodeRead,
    CodeWrite,
    CompRead,
    CompWrite,
    DashboardRead,
    EventBroadcast,
    EventCancel,
    EventClone,
    EventDataRead,
    EventDelete,
    EventFinancialReports,
    EventInterest,
    EventReportSubscriberDelete,
    EventReportSubscriberRead,
    EventReportSubscriberWrite,
    EventReports,
    EventScan,
    EventViewGuests,
    EventWrite,
    HoldRead,
    HoldWrite,
    ListingWrite,
    LootBoxWrite,
    NoteDelete,
    NoteRead,
    NoteWrite,
    OrderMakeExternalPayment,
    OrderRead,
    OrderReadOwn,
    OrderRefund,
    OrderRefundOverride,
    OrderResendConfirmation,
    OrgAdmin,
    OrgAdminUsers,
    OrgFans,
    OrgFinancialReports,
    OrgModifySettlementType,
    OrgRead,
    OrgReadEvents,
    OrgReports,
    OrgUsers,
    OrgWrite,
    OrgVenueDelete,
    OrgVenueRead,
    OrgVenueWrite,
    RarityWrite,
    RedeemTicket,
    RegionWrite,
    ReportAdmin,
    ScanReportRead,
    SettlementAdjustmentDelete,
    SettlementAdjustmentWrite,
    SettlementRead,
    SettlementReadEarly,
    SettlementWrite,
    TemporaryUserPromote,
    TransferCancel,
    TransferCancelAccepted,
    TransferCancelOwn,
    TransferRead,
    TransferReadOwn,
    TicketAdmin,
    TicketRead,
    TicketWrite,
    TicketWriteOwn,
    TicketTransfer,
    TicketTypeRead,
    TicketTypeWrite,
    TokenRefresh,
    UserRead,
    UserDelete,
    VenueWrite,
    WebSocketInitiate,
}

impl Serialize for Scopes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl fmt::Display for Scopes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Scopes::AnnouncementDelete => "announcement:delete",
            Scopes::AnnouncementRead => "announcement:read",
            Scopes::AnnouncementWrite => "announcement:write",
            Scopes::AnnouncementEngagementWrite => "announcement-engagement:write",
            Scopes::ArtistWrite => "artist:write",
            Scopes::BoxOfficeTicketRead => "box-office-ticket:read",
            Scopes::BoxOfficeTicketWrite => "box-office-ticket:write",
            Scopes::CodeRead => "code:read",
            Scopes::CodeWrite => "code:write",
            Scopes::CompRead => "comp:read",
            Scopes::CompWrite => "comp:write",
            Scopes::DashboardRead => "dashboard:read",
            Scopes::EventBroadcast => "event:broadcast",
            Scopes::EventCancel => "event:cancel",
            Scopes::EventClone => "event:clone",
            Scopes::EventDataRead => "event:data-read",
            Scopes::EventDelete => "event:delete",
            Scopes::EventWrite => "event:write",
            Scopes::EventFinancialReports => "event:financial-reports",
            Scopes::EventInterest => "event:interest",
            Scopes::EventReportSubscriberDelete => "event-report-subscriber:delete",
            Scopes::EventReportSubscriberRead => "event-report-subscriber:read",
            Scopes::EventReportSubscriberWrite => "event-report-subscriber:write",
            Scopes::EventReports => "event:reports",
            Scopes::EventScan => "event:scan",
            Scopes::EventViewGuests => "event:view-guests",
            Scopes::HoldRead => "hold:read",
            Scopes::HoldWrite => "hold:write",
            Scopes::ListingWrite => "listing:write",
            Scopes::LootBoxWrite => "loot-box:write",
            Scopes::NoteDelete => "note:delete",
            Scopes::NoteRead => "note:read",
            Scopes::NoteWrite => "note:write",
            Scopes::OrderRead => "order:read",
            Scopes::OrderMakeExternalPayment => "order:make-external-payment",
            Scopes::OrderReadOwn => "order:read-own",
            Scopes::OrderRefund => "order:refund",
            Scopes::OrderRefundOverride => "order:refund-override",
            Scopes::OrderResendConfirmation => "order:resend-confirmation",
            Scopes::OrgAdmin => "org:admin",
            Scopes::OrgModifySettlementType => "org:modify-settlement-type",
            Scopes::OrgRead => "org:read",
            Scopes::OrgReadEvents => "org:read-events",
            Scopes::OrgReports => "org:reports",
            Scopes::OrgFans => "org:fans",
            Scopes::OrgFinancialReports => "org:financial-reports",
            Scopes::OrgWrite => "org:write",
            Scopes::OrgAdminUsers => "org:admin-users",
            Scopes::OrgUsers => "org:users",
            Scopes::OrgVenueDelete => "org-venue:delete",
            Scopes::OrgVenueRead => "org-venue:read",
            Scopes::OrgVenueWrite => "org-venue:write",
            Scopes::RarityWrite => "rarity:write",
            Scopes::RedeemTicket => "redeem:ticket",
            Scopes::RegionWrite => "region:write",
            Scopes::ReportAdmin => "report:admin",
            Scopes::ScanReportRead => "scan-report:read",
            Scopes::SettlementAdjustmentDelete => "settlement-adjustment:delete",
            Scopes::SettlementAdjustmentWrite => "settlement-adjustment:write",
            Scopes::SettlementRead => "settlement:read",
            Scopes::SettlementReadEarly => "settlement:read-early",
            Scopes::SettlementWrite => "settlement:write",
            Scopes::TicketAdmin => "ticket:admin",
            Scopes::TicketRead => "ticket:read",
            Scopes::TicketWrite => "ticket:write",
            Scopes::TicketWriteOwn => "ticket:write-own",
            Scopes::TicketTransfer => "ticket:transfer",
            Scopes::TicketTypeRead => "ticket-type:read",
            Scopes::TicketTypeWrite => "ticket-type:write",
            Scopes::TransferCancel => "transfer:cancel",
            Scopes::TransferCancelAccepted => "transfer:cancel-accepted",
            Scopes::TransferCancelOwn => "transfer:cancel-own",
            Scopes::TransferRead => "transfer:read",
            Scopes::TransferReadOwn => "transfer:read-own",
            Scopes::TemporaryUserPromote => "temporary-user:promote",
            Scopes::TokenRefresh => "token:refresh",
            Scopes::UserRead => "user:read",
            Scopes::UserDelete => "user:delete",
            Scopes::VenueWrite => "venue:write",
            Scopes::WebSocketInitiate => "websocket:initiate",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for Scopes {
    type Err = EnumParseError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let s = match s {
            "announcement:delete" => Scopes::AnnouncementDelete,
            "announcement:read" => Scopes::AnnouncementRead,
            "announcement:write" => Scopes::AnnouncementWrite,
            "announcement-engagement:write" => Scopes::AnnouncementEngagementWrite,
            "artist:write" => Scopes::ArtistWrite,
            "box-office-ticket:read" => Scopes::BoxOfficeTicketRead,
            "box-office-ticket:write" => Scopes::BoxOfficeTicketWrite,
            "code:read" => Scopes::CodeRead,
            "code:write" => Scopes::CodeWrite,
            "comp:read" => Scopes::CompRead,
            "comp:write" => Scopes::CompWrite,
            "dashboard:read" => Scopes::DashboardRead,
            "event:broadcast" => Scopes::EventBroadcast,
            "event:cancel" => Scopes::EventCancel,
            "event:clone" => Scopes::EventClone,
            "event:data-read" => Scopes::EventDataRead,
            "event:delete" => Scopes::EventDelete,
            "event:write" => Scopes::EventWrite,
            "event:financial-reports" => Scopes::EventFinancialReports,
            "event:interest" => Scopes::EventInterest,
            "event-report-subscriber:delete" => Scopes::EventReportSubscriberDelete,
            "event-report-subscriber:read" => Scopes::EventReportSubscriberRead,
            "event-report-subscriber:write" => Scopes::EventReportSubscriberWrite,
            "event:reports" => Scopes::EventReports,
            "event:scan" => Scopes::EventScan,
            "event:view-guests" => Scopes::EventViewGuests,
            "hold:read" => Scopes::HoldRead,
            "hold:write" => Scopes::HoldWrite,
            "listing:write" => Scopes::ListingWrite,
            "loot-box:write" => Scopes::LootBoxWrite,
            "note:delete" => Scopes::NoteDelete,
            "note:read" => Scopes::NoteRead,
            "note:write" => Scopes::NoteWrite,
            "order:read" => Scopes::OrderRead,
            "order:make-external-payment" => Scopes::OrderMakeExternalPayment,
            "order:read-own" => Scopes::OrderReadOwn,
            "order:refund" => Scopes::OrderRefund,
            "order:refund-override" => Scopes::OrderRefundOverride,
            "order:resend-confirmation" => Scopes::OrderResendConfirmation,
            "org:admin" => Scopes::OrgAdmin,
            "org:modify-settlement-type" => Scopes::OrgModifySettlementType,
            "org:read" => Scopes::OrgRead,
            "org:read-events" => Scopes::OrgReadEvents,
            "org:reports" => Scopes::OrgReports,
            "org:fans" => Scopes::OrgFans,
            "org:financial-reports" => Scopes::OrgFinancialReports,
            "org:write" => Scopes::OrgWrite,
            "org:admin-users" => Scopes::OrgAdminUsers,
            "org:users" => Scopes::OrgUsers,
            "rarity:write" => Scopes::RarityWrite,
            "org-venue:delete" => Scopes::OrgVenueDelete,
            "org-venue:read" => Scopes::OrgVenueRead,
            "org-venue:write" => Scopes::OrgVenueWrite,
            "redeem:ticket" => Scopes::RedeemTicket,
            "region:write" => Scopes::RegionWrite,
            "report:admin" => Scopes::ReportAdmin,
            "scan-report:read" => Scopes::ScanReportRead,
            "settlement-adjustment:delete" => Scopes::SettlementAdjustmentDelete,
            "settlement-adjustment:write" => Scopes::SettlementAdjustmentWrite,
            "settlement:read" => Scopes::SettlementRead,
            "settlement:read-early" => Scopes::SettlementReadEarly,
            "settlement:write" => Scopes::SettlementWrite,
            "temporary-user:promote" => Scopes::TemporaryUserPromote,
            "ticket:admin" => Scopes::TicketAdmin,
            "ticket:read" => Scopes::TicketRead,
            "ticket:write" => Scopes::TicketWrite,
            "ticket:write-own" => Scopes::TicketWriteOwn,
            "ticket:transfer" => Scopes::TicketTransfer,
            "ticket-type:read" => Scopes::TicketTypeRead,
            "ticket-type:write" => Scopes::TicketTypeWrite,
            "token:refresh" => Scopes::TokenRefresh,
            "transfer:cancel" => Scopes::TransferCancel,
            "transfer:cancel-accepted" => Scopes::TransferCancelAccepted,
            "transfer:cancel-own" => Scopes::TransferCancelOwn,
            "transfer:read" => Scopes::TransferRead,
            "transfer:read-own" => Scopes::TransferReadOwn,
            "user:read" => Scopes::UserRead,
            "user:delete" => Scopes::UserDelete,
            "venue:write" => Scopes::VenueWrite,
            "websocket:initiate" => Scopes::WebSocketInitiate,
            _ => {
                return Err(EnumParseError {
                    message: "Could not parse value".to_string(),
                    enum_type: "Scopes".to_string(),
                    value: s.to_string(),
                });
            }
        };
        Ok(s)
    }
}

pub fn get_scopes(roles: Vec<Roles>, additional_scopes: Option<AdditionalOrgMemberScopes>) -> Vec<Scopes> {
    let mut scopes: Vec<Scopes> = roles.into_iter().flat_map(|r| get_scopes_for_role(r)).collect();
    if let Some(extra_scopes) = additional_scopes {
        scopes.append(&mut extra_scopes.additional.clone());
        scopes.retain(|&scope| !extra_scopes.revoked.contains(&scope));
    }
    scopes.sort();
    scopes.dedup();
    scopes
}

fn get_scopes_for_role(role: Roles) -> Vec<Scopes> {
    use models::Roles::*;
    let mut roles = match role {
        User => {
            let roles = vec![
                Scopes::EventInterest,
                Scopes::OrderReadOwn,
                Scopes::TicketTransfer,
                Scopes::TicketWriteOwn,
                Scopes::TransferCancelOwn,
                Scopes::TransferReadOwn,
                Scopes::ListingWrite,
            ];
            roles
        }
        DoorPerson => {
            let roles = vec![
                Scopes::RedeemTicket,
                Scopes::OrgReadEvents,
                Scopes::HoldRead,
                Scopes::NoteRead,
                Scopes::NoteWrite,
                Scopes::EventScan,
                Scopes::TicketRead,
                Scopes::EventViewGuests,
                Scopes::CodeRead,
                Scopes::OrderRead,
                Scopes::DashboardRead,
                Scopes::WebSocketInitiate,
            ];
            roles
        }
        PrismIntegration => {
            let roles = vec![Scopes::EventDataRead];
            roles
        }
        OrgBoxOffice => {
            let mut roles = vec![
                Scopes::AnnouncementEngagementWrite,
                Scopes::BoxOfficeTicketRead,
                Scopes::EventViewGuests,
                Scopes::OrderMakeExternalPayment,
                Scopes::OrderResendConfirmation,
                Scopes::OrgFans,
            ];

            roles.extend(get_scopes_for_role(Roles::DoorPerson));
            roles
        }
        PromoterReadOnly => {
            let roles = vec![
                Scopes::AnnouncementEngagementWrite,
                Scopes::CodeRead,
                Scopes::CompRead,
                Scopes::DashboardRead,
                Scopes::EventViewGuests,
                Scopes::EventInterest,
                Scopes::HoldRead,
                Scopes::NoteRead,
                Scopes::OrderRead,
                Scopes::OrgReadEvents,
                Scopes::ScanReportRead,
                Scopes::TicketRead,
                Scopes::TicketTypeRead,
                Scopes::TransferRead,
                Scopes::WebSocketInitiate,
            ];
            roles
        }
        Promoter => {
            let mut roles = vec![
                Scopes::CodeWrite,
                Scopes::CompWrite,
                // Scopes::EventFinancialReports,
                // Scopes::EventReports,
                // To be updated later
                Scopes::EventDelete,
                Scopes::EventWrite,
                Scopes::NoteWrite,
                Scopes::TicketTypeRead,
                Scopes::TicketTypeWrite,
                Scopes::HoldWrite,
                // Scopes::OrderRefund,
                Scopes::TransferCancel,
                Scopes::WebSocketInitiate,
            ];
            roles.extend(get_scopes_for_role(Roles::PromoterReadOnly));
            roles
        }
        OrgMember => {
            let mut roles = vec![
                Scopes::AnnouncementEngagementWrite,
                Scopes::ArtistWrite,
                Scopes::BoxOfficeTicketRead,
                Scopes::BoxOfficeTicketWrite,
                Scopes::CodeRead,
                Scopes::CodeWrite,
                Scopes::CompRead,
                Scopes::CompWrite,
                Scopes::DashboardRead,
                Scopes::EventBroadcast,
                Scopes::EventCancel,
                Scopes::EventClone,
                Scopes::EventDelete,
                Scopes::EventReportSubscriberDelete,
                Scopes::EventReportSubscriberRead,
                Scopes::EventReportSubscriberWrite,
                Scopes::EventScan,
                Scopes::EventViewGuests,
                Scopes::EventWrite,
                Scopes::HoldRead,
                Scopes::HoldWrite,
                Scopes::LootBoxWrite,
                Scopes::NoteRead,
                Scopes::NoteWrite,
                Scopes::OrderRead,
                Scopes::OrderRefund,
                Scopes::OrderResendConfirmation,
                Scopes::OrgRead,
                Scopes::OrgReadEvents,
                Scopes::OrgFans,
                Scopes::RarityWrite,
                Scopes::RedeemTicket,
                Scopes::ScanReportRead,
                Scopes::TicketAdmin,
                Scopes::TicketRead,
                Scopes::TicketTypeRead,
                Scopes::TicketTypeWrite,
                Scopes::TransferRead,
                Scopes::TransferCancel,
                Scopes::VenueWrite,
                Scopes::WebSocketInitiate,
            ];
            roles.extend(get_scopes_for_role(Roles::User));
            roles
        }
        OrgAdmin => {
            let mut roles = vec![
                Scopes::OrgWrite,
                Scopes::UserRead,
                Scopes::OrgUsers,
                Scopes::EventDataRead,
                Scopes::EventFinancialReports,
                Scopes::EventReports,
                Scopes::NoteDelete,
                Scopes::OrgReports,
                Scopes::SettlementRead,
                Scopes::TicketWrite,
            ];
            roles.extend(get_scopes_for_role(OrgMember));
            roles.extend(get_scopes_for_role(Roles::OrgBoxOffice));
            roles
        }
        OrgOwner => {
            let mut roles = vec![Scopes::OrgAdminUsers];
            roles.extend(get_scopes_for_role(Roles::OrgAdmin));
            roles
        }
        Admin => {
            let mut roles = vec![
                Scopes::AnnouncementDelete,
                Scopes::AnnouncementRead,
                Scopes::AnnouncementWrite,
                Scopes::OrderRefundOverride,
                Scopes::OrgAdmin,
                Scopes::OrgFinancialReports,
                Scopes::OrgModifySettlementType,
                Scopes::OrgVenueDelete,
                Scopes::OrgVenueRead,
                Scopes::OrgVenueWrite,
                Scopes::RegionWrite,
                Scopes::ReportAdmin,
                Scopes::SettlementAdjustmentDelete,
                Scopes::SettlementAdjustmentWrite,
                Scopes::SettlementReadEarly,
                Scopes::SettlementWrite,
                Scopes::TransferCancelAccepted,
                Scopes::UserDelete,
            ];
            roles.extend(get_scopes_for_role(OrgOwner));
            roles
        }
        Super => {
            let mut roles = vec![];
            roles.extend(get_scopes_for_role(Admin));
            roles
        }
    };
    roles.sort();
    roles.dedup();

    roles
}

#[test]
fn get_scopes_for_role_test() {
    let res = get_scopes_for_role(Roles::OrgOwner);
    assert_equiv!(
        vec![
            Scopes::AnnouncementEngagementWrite,
            Scopes::ArtistWrite,
            Scopes::BoxOfficeTicketRead,
            Scopes::BoxOfficeTicketWrite,
            Scopes::CodeRead,
            Scopes::CodeWrite,
            Scopes::CompRead,
            Scopes::CompWrite,
            Scopes::DashboardRead,
            Scopes::EventBroadcast,
            Scopes::EventCancel,
            Scopes::EventClone,
            Scopes::EventDataRead,
            Scopes::EventDelete,
            Scopes::EventFinancialReports,
            Scopes::EventInterest,
            Scopes::EventReportSubscriberDelete,
            Scopes::EventReportSubscriberRead,
            Scopes::EventReportSubscriberWrite,
            Scopes::EventReports,
            Scopes::EventScan,
            Scopes::EventViewGuests,
            Scopes::EventWrite,
            Scopes::HoldRead,
            Scopes::HoldWrite,
            Scopes::ListingWrite,
            Scopes::LootBoxWrite,
            Scopes::NoteDelete,
            Scopes::NoteRead,
            Scopes::NoteWrite,
            Scopes::OrderMakeExternalPayment,
            Scopes::OrderRead,
            Scopes::OrderReadOwn,
            Scopes::OrderRefund,
            Scopes::OrderResendConfirmation,
            Scopes::OrgAdminUsers,
            Scopes::OrgFans,
            Scopes::OrgRead,
            Scopes::OrgReports,
            Scopes::OrgUsers,
            Scopes::OrgWrite,
            Scopes::RarityWrite,
            Scopes::RedeemTicket,
            Scopes::ScanReportRead,
            Scopes::SettlementRead,
            Scopes::TicketAdmin,
            Scopes::TicketRead,
            Scopes::TicketWrite,
            Scopes::TicketWriteOwn,
            Scopes::TicketTransfer,
            Scopes::TicketTypeRead,
            Scopes::TicketTypeWrite,
            Scopes::TransferCancel,
            Scopes::TransferCancelOwn,
            Scopes::TransferRead,
            Scopes::TransferReadOwn,
            Scopes::UserRead,
            Scopes::VenueWrite,
            Scopes::WebSocketInitiate,
            Scopes::OrgReadEvents
        ],
        res
    );
}

#[test]
fn scopes_to_string() {
    assert_eq!("org:admin".to_string(), Scopes::OrgAdmin.to_string());
}

#[test]
fn get_scopes_test() {
    let mut res = get_scopes(vec![Roles::OrgOwner], None)
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    res.sort();
    assert_equiv!(
        vec![
            "announcement-engagement:write",
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:clone",
            "event:data-read",
            "event:delete",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "event-report-subscriber:delete",
            "event-report-subscriber:read",
            "event-report-subscriber:write",
            "hold:read",
            "hold:write",
            "listing:write",
            "loot-box:write",
            "note:delete",
            "note:read",
            "note:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "order:resend-confirmation",
            "org:admin-users",
            "org:fans",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "rarity:write",
            "redeem:ticket",
            "scan-report:read",
            "settlement:read",
            "ticket-type:read",
            "ticket-type:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket:write",
            "ticket:write-own",
            "transfer:cancel-own",
            "transfer:cancel",
            "transfer:read-own",
            "transfer:read",
            "user:read",
            "venue:write",
            "websocket:initiate",
        ],
        res
    );
    let mut res = get_scopes(vec![Roles::Admin], None)
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    res.sort();
    assert_equiv!(
        vec![
            "announcement:delete",
            "announcement:read",
            "announcement:write",
            "announcement-engagement:write",
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:clone",
            "event:data-read",
            "event:delete",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "event-report-subscriber:delete",
            "event-report-subscriber:read",
            "event-report-subscriber:write",
            "hold:read",
            "hold:write",
            "note:delete",
            "note:read",
            "note:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "order:refund-override",
            "order:resend-confirmation",
            "org:admin",
            "org:admin-users",
            "org:fans",
            "org:financial-reports",
            "org:modify-settlement-type",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "org-venue:delete",
            "org-venue:read",
            "org-venue:write",
            "redeem:ticket",
            "region:write",
            "report:admin",
            "scan-report:read",
            "settlement-adjustment:delete",
            "settlement-adjustment:write",
            "settlement:read",
            "settlement:read-early",
            "settlement:write",
            "ticket-type:read",
            "ticket-type:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket:write",
            "ticket:write-own",
            "transfer:cancel",
            "transfer:cancel-accepted",
            "transfer:cancel-own",
            "transfer:read",
            "transfer:read-own",
            "user:read",
            "user:delete",
            "venue:write",
            "websocket:initiate",
        ],
        res
    );

    let mut res = get_scopes(vec![Roles::Super], None)
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    res.sort();
    assert_equiv!(
        vec![
            "announcement:delete",
            "announcement:read",
            "announcement:write",
            "announcement-engagement:write",
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:clone",
            "event:data-read",
            "event:delete",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "event-report-subscriber:delete",
            "event-report-subscriber:read",
            "event-report-subscriber:write",
            "hold:read",
            "hold:write",
            "note:delete",
            "note:read",
            "note:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "order:refund-override",
            "order:resend-confirmation",
            "org:admin",
            "org:admin-users",
            "org:fans",
            "org:financial-reports",
            "org:modify-settlement-type",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "org-venue:delete",
            "org-venue:read",
            "org-venue:write",
            "redeem:ticket",
            "region:write",
            "report:admin",
            "scan-report:read",
            "settlement-adjustment:delete",
            "settlement-adjustment:write",
            "settlement:read",
            "settlement:read-early",
            "settlement:write",
            "ticket-type:read",
            "ticket-type:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket:write",
            "ticket:write-own",
            "transfer:cancel",
            "transfer:cancel-accepted",
            "transfer:cancel-own",
            "transfer:read",
            "transfer:read-own",
            "user:read",
            "user:delete",
            "venue:write",
            "websocket:initiate",
        ],
        res
    );

    let res = get_scopes(vec![Roles::OrgOwner, Roles::Admin], None)
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    assert_equiv!(
        vec![
            "announcement:delete",
            "announcement:read",
            "announcement:write",
            "announcement-engagement:write",
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:clone",
            "event:data-read",
            "event:delete",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "event-report-subscriber:delete",
            "event-report-subscriber:read",
            "event-report-subscriber:write",
            "hold:read",
            "hold:write",
            "note:delete",
            "note:read",
            "note:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "order:refund-override",
            "order:resend-confirmation",
            "org:admin",
            "org:admin-users",
            "org:fans",
            "org:financial-reports",
            "org:modify-settlement-type",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "org-venue:delete",
            "org-venue:read",
            "org-venue:write",
            "redeem:ticket",
            "region:write",
            "report:admin",
            "scan-report:read",
            "settlement-adjustment:delete",
            "settlement-adjustment:write",
            "settlement:read",
            "settlement:read-early",
            "settlement:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket:write",
            "ticket:write-own",
            "ticket-type:read",
            "ticket-type:write",
            "transfer:cancel",
            "transfer:cancel-accepted",
            "transfer:cancel-own",
            "transfer:read",
            "transfer:read-own",
            "user:delete",
            "user:read",
            "venue:write",
            "websocket:initiate",
        ],
        res
    );

    let res = get_scopes(
        vec![Roles::OrgOwner],
        Some(AdditionalOrgMemberScopes {
            additional: vec![Scopes::SettlementAdjustmentDelete],
            revoked: vec![Scopes::EventWrite],
        }),
    )
    .iter()
    .map(|i| i.to_string())
    .collect::<Vec<String>>();
    assert_equiv!(
        vec![
            "announcement-engagement:write",
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:clone",
            "event:data-read",
            "event:delete",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event-report-subscriber:delete",
            "event-report-subscriber:read",
            "event-report-subscriber:write",
            "hold:read",
            "hold:write",
            "note:delete",
            "note:read",
            "note:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "order:resend-confirmation",
            "org:admin-users",
            "org:fans",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "redeem:ticket",
            "scan-report:read",
            "settlement-adjustment:delete",
            "settlement:read",
            "ticket-type:read",
            "ticket-type:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket:write",
            "ticket:write-own",
            "transfer:cancel-own",
            "transfer:cancel",
            "transfer:read-own",
            "transfer:read",
            "user:read",
            "venue:write",
            "websocket:initiate",
        ],
        res
    );
}

#[test]
fn from_str() {
    let s: Scopes = "ticket:read".parse().unwrap();
    assert_eq!(Scopes::TicketRead, s);
}
