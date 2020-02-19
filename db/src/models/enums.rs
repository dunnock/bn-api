use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::*;
use std::cmp::Ordering;
use std::fmt;
use std::io::Write;
use std::str;
use std::str::FromStr;
use utils::errors::EnumParseError;

macro_rules! string_enum {
    ($name:ident [$($value:ident),+]) => {

        #[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Eq, Hash, FromSqlRow, AsExpression)]
        #[sql_type = "Text"]
        pub enum $name {
            $(
                $value,
            )*
        }

        impl Ord for $name {
            fn cmp(&self, other: &$name) -> Ordering {
                self.to_string().cmp(&other.to_string())
            }
        }

        impl PartialOrd  for $name {
             fn partial_cmp(&self, other: &$name) -> Option<Ordering> {
                 Some(self.cmp(&other))
             }
        }

        impl ToSql<Text, Pg> for $name {
            fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
                match *self {
                    $(
                      $name::$value => out.write_all(stringify!($value).as_bytes())?,
                    )*
                }
                Ok(IsNull::No)
            }
        }

        impl FromSql<Text, Pg> for $name {
            fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
                let s = str::from_utf8(not_none!(bytes))?;
                s.parse().map_err(|_| format!("Unrecognized enum variant:{}", s).into())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
             let s = match self {
                  $(
                    $name::$value => stringify!($value),
                   )*
                };
                write!(f, "{}", s)
            }
        }

        impl FromStr for $name {
            type Err = EnumParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
               $(
                  if s.eq_ignore_ascii_case(stringify!($value)) {
                     return Ok($name::$value);
                  }
               )*

               Err(EnumParseError {
                          message: "Could not parse value".to_string(),
                          enum_type: stringify!($name).to_string(),
                          value: s.to_string(),
                      })
            }
        }
    }
}

string_enum! { ActivityType [Purchase, Transfer, CheckIn,Refund, Note]}
string_enum! { AssetStatus [Unsynced] }
string_enum! { BroadcastAudience [ PeopleAtTheEvent, TicketHolders  ]}
string_enum! { CartItemStatus [CodeExpired, HoldExpired, TicketNullified, TicketNotReserved, Valid] }
string_enum! { CheckInSource [GuestList, Scanned] }
string_enum! { CodeTypes [Access, Discount] }
string_enum! { CommunicationChannelType [Email, Sms, Push, Webhook]}
string_enum! { CommunicationType [EmailTemplate, Sms, Push, Webhook]}
string_enum! { DomainEventTypes [
    CodeCreated,
    CodeDeleted,
    CodeUpdated,
    EventArtistCreated,
    EventArtistAdded,
    EventCancelled,
    EventCloned,
    EventCreated,
    EventDeleted,
    EventInterestCreated,
    EventPublished,
    EventReportSubscriberCreated,
    EventReportSubscriberDeleted,
    EventUpdated,
    EventUnpublished,
    ExternalLoginCreated,
    ExternalLoginDeleted,
    FeeScheduleCreated,
    GenresUpdated,
    HoldCreated,
    HoldDeleted,
    HoldQuantityChanged,
    OrderBehalfOfUserChanged,
    OrderCompleted,
    OrderCreated,
    OrderRefund,
    OrderResendConfirmationTriggered,
    OrderRetargetingEmailTriggered,
    OrderStatusUpdated,
    OrderUpdated,
    OrganizationCreated,
    NoteCreated,
    NoteDeleted,
    PaymentCancelled,
    PaymentCreated,
    PaymentCompleted,
    PaymentRefund,
    PaymentProviderIPN,
    PaymentMethodCreated,
    PaymentMethodUpdated,
    PaymentUpdated,
    UserCreated,
    UserDisabled,
    UserLogin,
    UserRegistration,
    UserUpdated,
    LostPassword,
    PurchaseCompleted,
    PushNotificationTokenCreated,
    SettlementReportProcessed,
    TransferTicketDripSourceSent,
    TransferTicketDripDestinationSent,
    TransferTicketCancelled,
    TransferTicketCompleted,
    TransferTicketStarted,
    TrackingDataUpdated,
    TemporaryUserCreated,
    TicketInstanceAddedToHold,
    TicketInstanceNullified,
    TicketInstancePurchased,
    TicketInstanceRedeemed,
    TicketInstanceReleasedFromHold,
    TicketInstanceUpdated,
    TicketPricingAdded,
    TicketPricingCreated,
    TicketPricingDeleted,
    TicketPricingSalesStarted,
    TicketPricingUpdated,
    TicketTypeCreated,
    TicketTypeSalesStarted,
    TicketTypeSoldOut,
    TicketTypeUpdated
]}
string_enum! { DomainActionTypes [
    BroadcastPushNotification,
    // Email/SMS/Push Communication
    Communication,
    PaymentProviderIPN,
    ProcessSettlementReport,
    ProcessTransferDrip,
    RegenerateDripActions,
    RetargetAbandonedOrders,
    SendAutomaticReportEmails,
    SendPurchaseCompletedCommunication,
    SubmitSitemapToSearchEngines,
    UpdateGenres
]}
string_enum! { BroadcastStatus [Pending, InProgress, Completed, Cancelled]}
string_enum! { BroadcastChannel [PushNotification, Email]}
string_enum! { BroadcastType [Custom, LastCall]}
string_enum! { DomainActionStatus [Pending, RetriesExceeded, Errored, Success, Cancelled]}
string_enum! { EmailProvider [Sendgrid, CustomerIo]}
string_enum! { Environment [Development, Production, Staging, Test]}
string_enum! { EventStatus [Draft,Closed,Published,Offline]}
string_enum! { EventSearchSortField [ Name, EventStart]}
string_enum! { EventOverrideStatus [PurchaseTickets,SoldOut,OnSaleSoon,TicketsAtTheDoor,Free,Rescheduled,Cancelled,OffSale,Ended]}
string_enum! { EventTypes [ Music, Conference, Art, Culinary, Comedy, Sports, Tech, Other]}
string_enum! { ExternalPaymentType [Cash, CreditCard, Voucher]}
string_enum! { FanSortField [FirstName, LastName, Email, Phone, OrganizationId, UserCreated, Orders, FirstOrder, LastOrder, Revenue, FirstInteracted, LastInteracted] }
string_enum! { HistoryType [Purchase]}
string_enum! { HoldTypes [Discount, Comp] }
string_enum! { OrderStatus [Cancelled, Draft, Paid, PendingPayment] }
string_enum! { OrderItemTypes [Tickets, PerUnitFees, EventFees, Discount, CreditCardFees]}
string_enum! { OrderTypes [Cart, BackOffice] }
string_enum! { PaymentMethods [CreditCard, External, Free, Provider] }
string_enum! { PaymentProviders [External, Globee, Free, Stripe] }
string_enum! { PaymentStatus [Authorized, Completed, Requested, Refunded, Unpaid, PendingConfirmation, Cancelled, Draft, Unknown, PendingIpn] }
string_enum! { PastOrUpcoming [Past,Upcoming]}
string_enum! { Platforms [Web, App, BoxOffice]}
string_enum! { ReportTypes [TicketCounts]}
string_enum! { Roles [Admin, DoorPerson, OrgAdmin, OrgBoxOffice, OrgMember, OrgOwner, PrismIntegration, Promoter, PromoterReadOnly, User, Super] }
string_enum! { SettlementStatus[PendingSettlement, RequiresAudit, SettledInFull] }
string_enum! { SettlementTypes [Rolling, PostEvent]}
string_enum! { SettlementAdjustmentTypes [ManualCredit, ManualDeduction, Chargeback]}
string_enum! { SettlementEntryTypes [EventFees, TicketType]}
string_enum! { SlugTypes[ Event, Organization, Venue, City, Genre, CityGenre ] }
string_enum! { SortingDir[ Asc, Desc ] }
string_enum! { SourceOrDestination [Destination,Source]}
string_enum! { Tables [
    Artists, Broadcasts, Codes, DomainEventPublishers, Events, EventArtists, EventReportSubscribers, ExternalLogins, FeeSchedules,
    Holds, Orders, Organizations, Notes, Payments, PaymentMethods, PushNotificationTokens, TemporaryUsers, TicketInstances, TicketTypes,
    TicketPricing, Transfers, Users, Venues, Genres
] }
string_enum! { TicketInstanceStatus [Available, Reserved, Purchased, Redeemed, Nullified]}
string_enum! { TicketPricingStatus [Published, Deleted, Default] }
string_enum! { TicketTypeEndDateType [DoorTime, EventEnd, EventStart, Manual] }
string_enum! { TicketTypeStatus [NoActivePricing, Published, SoldOut, OnSaleSoon, SaleEnded, Cancelled, Deleted] }
string_enum! { TicketTypeVisibility [ Always, Hidden, WhenAvailable ]}
string_enum! { TransferMessageType [Email, Phone] }
string_enum! { TransferStatus [Pending, Cancelled, Completed, EventEnded] }
string_enum! { WebhookAdapters [CustomerIo]}

impl Roles {
    pub fn get_event_limited_roles() -> Vec<Roles> {
        vec![Roles::Promoter, Roles::PromoterReadOnly]
    }
}

impl Default for EventStatus {
    fn default() -> EventStatus {
        EventStatus::Draft
    }
}

impl Default for EventTypes {
    fn default() -> EventTypes {
        EventTypes::Music
    }
}

impl Default for BroadcastType {
    fn default() -> BroadcastType {
        BroadcastType::LastCall
    }
}

impl Default for BroadcastStatus {
    fn default() -> BroadcastStatus {
        BroadcastStatus::Pending
    }
}

impl Default for BroadcastChannel {
    fn default() -> BroadcastChannel {
        BroadcastChannel::PushNotification
    }
}

impl Default for BroadcastAudience {
    fn default() -> BroadcastAudience {
        BroadcastAudience::PeopleAtTheEvent
    }
}

impl Tables {
    pub fn table_name(&self) -> String {
        self.to_string().to_ascii_lowercase()
    }
}

impl OrderItemTypes {
    pub fn is_fee(self) -> bool {
        self == OrderItemTypes::PerUnitFees
            || self == OrderItemTypes::EventFees
            || self == OrderItemTypes::CreditCardFees
    }
}

#[test]
fn get_event_limited_roles() {
    assert_eq!(
        Roles::get_event_limited_roles(),
        vec![Roles::Promoter, Roles::PromoterReadOnly]
    );
}

#[test]
fn display() {
    assert_eq!(Roles::Admin.to_string(), "Admin");
    assert_eq!(Roles::OrgAdmin.to_string(), "OrgAdmin");
    assert_eq!(Roles::OrgMember.to_string(), "OrgMember");
    assert_eq!(Roles::OrgOwner.to_string(), "OrgOwner");
    assert_eq!(Roles::OrgBoxOffice.to_string(), "OrgBoxOffice");
    assert_eq!(Roles::DoorPerson.to_string(), "DoorPerson");
    assert_eq!(Roles::Promoter.to_string(), "Promoter");
    assert_eq!(Roles::PromoterReadOnly.to_string(), "PromoterReadOnly");
    assert_eq!(Roles::User.to_string(), "User");
}

#[test]
fn parse() {
    assert_eq!(Roles::Admin, "Admin".parse().unwrap());
    assert_eq!(Roles::OrgMember, "OrgMember".parse().unwrap());
    assert_eq!(Roles::OrgOwner, "OrgOwner".parse().unwrap());
    assert_eq!(Roles::OrgBoxOffice, "OrgBoxOffice".parse().unwrap());
    assert!("Invalid Role".parse::<Roles>().is_err());
}

#[test]
fn to_table_name() {
    assert_eq!(Tables::Events.table_name(), "events");
}
