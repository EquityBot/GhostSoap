// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The law table: each lever's legal basis, deadline and charge rule, with citations.
//!
//! Every value was read in the primary text on 25 October 2026: the Privacy Act 1988
//! compilation C2026C00227 (in force from 4 June 2026), the Spam Act 2003 compilation
//! C2016C00614, the Spam Regulations 2021 (F2021L00285) and the OAIC's APP guidelines
//! (chapters 7, 12 and 13). Adding a lever needs a `Lever` variant, a row here and a letter;
//! the match below makes a missing row a compile error.

use crate::core::register::Lever;

/// The legal texts the table cites, with the version read.
pub const SOURCES: [&str; 4] = [
    "Privacy Act 1988 (Cth), compilation C2026C00227, in force from 4 June 2026",
    "Spam Act 2003 (Cth), compilation C2016C00614",
    "Spam Regulations 2021 (Cth), F2021L00285",
    "OAIC, Australian Privacy Principles guidelines, chapters 7, 12 and 13, read 25 October 2026",
];

/// The day the status lines below were read in their primary sources, unless a line gives
/// its own date.
pub const STATUS_READ: &str = "5 November 2026";

/// Where the law stands around the table: the sources are in the plan, section 3. A
/// statement about tranche 2 describes a draft and must say so.
pub const STATUS: [&str; 7] = [
    "In force: Privacy Act 1988 compilation C2026C00227. The Federal Register ends that version on 10 December 2026 and has not yet registered the next compilation.",
    "From 10 December 2026: APP 1.7 to 1.9 require a privacy policy to describe certain automated decisions. That is a transparency duty: it gives you no request to make, so no lever uses it.",
    "Children's Online Privacy Code (read 6 November 2026): s 26GC(10) requires the Commissioner to develop and register it within 24 months of Royal Assent on 10 December 2024, which the OAIC gives as 10 December 2026. The OAIC consulted on an Exposure Draft until 5 June 2026, and the Federal Register lists no such code yet. It would bind APP entities providing certain online services likely to be accessed by children. No lever uses it.",
    "From 1 January 2027: the Interactive Gambling Amendment (Gambling Reform) Act 2026, sch 1 items 46 and 47, adds s 33C(1)(h) and inserts 'and subsection 62P(2)' in APP 7.8(aa). APP 7.8 limits APP 7, which the app7 levers rest on, so reread it then. No lever changes on the face of the items.",
    "Tranche 2: the Privacy Amendment (Personal Data Protection) Bill 2026 is an Exposure Draft, not law, and may change. Consultation closed on 18 September 2026, and the draft's commencement table is blank.",
    "The Exposure Draft would replace APP 7 and add a right to erasure against large digital platforms only. Neither applies today: every lever above rests on the law in force.",
    "OAIC complaints: as at February 2026 the OAIC said it was unlikely to substantially progress a new complaint for some 6 to 12 months after lodgement, unless it finds exceptional circumstances, and it need not investigate every complaint. Where a recognised external dispute resolution scheme covers the business, such as the TIO or AFCA, generally try that scheme first, but the OAIC may decide not to investigate a complaint made more than 12 months after you became aware of the act or practice (s 41(1)(c)).",
];

/// When the entity must act.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deadline {
    /// A fixed number of days after the request is made.
    Days(u16),
    /// Business days in the place the message was sent to (Spam Act Sch 2 cl 6(2)).
    BusinessDays(u16),
    /// 'A reasonable period', with the OAIC's general guide in days where it gives one.
    Reasonable { guide_days: Option<u16> },
    /// The law sets no time for a response.
    NotStated,
}

/// One row of the law table.
#[derive(Debug, Clone, Copy)]
pub struct Law {
    /// What the user asks for, as a heading.
    pub title: &'static str,
    /// The provisions that give the request and its deadline.
    pub provision: &'static str,
    pub deadline: Deadline,
    /// The deadline as a letter states it, with its citation.
    pub deadline_text: &'static str,
    /// Whether the entity may charge, with its citation.
    pub charge: &'static str,
}

/// The law table row for `lever`.
#[allow(clippy::too_many_lines)] // one arm per lever; splitting it would only scatter the table
pub fn law(lever: Lever) -> Law {
    const APP7_FREE: &str =
        "No charge may be made for making or giving effect to the request (APP 7.7).";
    const APP13_FREE: &str = "No charge may be made for the request, the correction or associating a statement (APP 13.5(b)).";
    match lever {
        Lever::App7NoMarketing => Law {
            title: "Stop sending me direct marketing",
            provision: "APP 7.6(c), 7.7(a)",
            deadline: Deadline::Reasonable {
                guide_days: Some(30),
            },
            deadline_text: "within a reasonable period after the request is made (APP 7.7(a)), which the OAIC's APP guidelines say would generally be no more than 30 days (paragraph 7.37)",
            charge: APP7_FREE,
        },
        Lever::App7NoFacilitation => Law {
            title: "Stop using or disclosing my information for other organisations' direct marketing",
            provision: "APP 7.6(d), 7.7(a)",
            deadline: Deadline::Reasonable {
                guide_days: Some(30),
            },
            deadline_text: "within a reasonable period after the request is made (APP 7.7(a)), which the OAIC's APP guidelines say would be no more than 30 days (paragraph 7.41)",
            charge: APP7_FREE,
        },
        Lever::App7Source => Law {
            title: "Tell me where you got my information",
            provision: "APP 7.6(e), 7.7(b)",
            deadline: Deadline::Reasonable {
                guide_days: Some(30),
            },
            deadline_text: "within a reasonable period after the request is made, unless it is impracticable or unreasonable to do so (APP 7.7(b)); the OAIC's APP guidelines say a reasonable period would generally be 30 days unless special circumstances apply (paragraph 7.47)",
            charge: APP7_FREE,
        },
        Lever::App11Destroy => Law {
            title: "Destroy or de-identify information you no longer need",
            provision: "APP 11.2",
            deadline: Deadline::NotStated,
            deadline_text: "APP 11.2 sets no timeframe; it requires reasonable steps to destroy or de-identify personal information an entity no longer needs for a permitted purpose",
            charge: "APP 11.2 is an obligation on the entity, not a request; it provides for no charge.",
        },
        Lever::App12 => Law {
            title: "Give me access to the personal information you hold about me",
            provision: "APP 12.1, 12.4(a)(ii)",
            deadline: Deadline::Reasonable {
                guide_days: Some(30),
            },
            deadline_text: "within a reasonable period after the request is made (APP 12.4(a)(ii)), which the OAIC's APP guidelines say should not exceed 30 calendar days (paragraph 12.67)",
            charge: "No charge may be made for making the request, and any charge for giving access must not be excessive (APP 12.8).",
        },
        Lever::App13 => Law {
            title: "Correct personal information that is wrong",
            provision: "APP 13.1, 13.5(a)(ii)",
            deadline: Deadline::Reasonable {
                guide_days: Some(30),
            },
            deadline_text: "within a reasonable period after the request is made (APP 13.5(a)(ii)), which the OAIC's APP guidelines say should not exceed 30 calendar days (paragraph 13.63)",
            charge: APP13_FREE,
        },
        Lever::App13NotifyRecipients => Law {
            title: "Notify the entities you disclosed my information to if you correct it",
            provision: "APP 13.2",
            deadline: Deadline::NotStated,
            deadline_text: "APP 13.2 sets no timeframe; it requires reasonable steps to give the notification unless it is impracticable or unlawful to do so",
            charge: "APP 13.2 sets no charge rule of its own; APP 13.5(b) bars charging for a correction request or a statement request.",
        },
        Lever::App13Statement => Law {
            title: "Associate my statement with the information if you refuse to correct it",
            provision: "APP 13.4, 13.5(a)(ii)",
            deadline: Deadline::Reasonable {
                guide_days: Some(30),
            },
            deadline_text: "within a reasonable period after the request is made (APP 13.5(a)(ii)), which the OAIC's APP guidelines say should not exceed 30 calendar days (paragraph 13.63)",
            charge: APP13_FREE,
        },
        Lever::S20rAccess => Law {
            title: "Give me access to my credit reporting information",
            provision: "Privacy Act s 20R(1), (3)",
            deadline: Deadline::Days(10),
            deadline_text: "within a reasonable period, but not longer than 10 days, after the request is made (section 20R(3))",
            charge: "No charge may be made if no access request was made to the body in the previous 3 months (section 20R(5)); otherwise any charge must not be excessive and must not apply to making the request (section 20R(6)).",
        },
        Lever::S20tCorrection => Law {
            title: "Correct my credit information",
            provision: "Privacy Act s 20T(1), (2)",
            deadline: Deadline::Days(30),
            deadline_text: "within 30 days from the day the request is made, or a longer period I agree to in writing (section 20T(2))",
            charge: "No charge may be made for the request or the correction (section 20T(5)).",
        },
        Lever::S20kBan => Law {
            title: "Do not use or disclose my credit reporting information during a ban period",
            provision: "Privacy Act s 20K(1)–(3)",
            deadline: Deadline::NotStated,
            deadline_text: "the ban period starts when the request is made and ends 21 days after the day it is made, unless extended (section 20K(3))",
            charge: "No charge may be made for the request or for giving effect to it (section 20K(6)).",
        },
        Lever::S21tAccess => Law {
            title: "Give me access to my credit eligibility information",
            provision: "Privacy Act s 21T(1), (3)",
            deadline: Deadline::Reasonable { guide_days: None },
            deadline_text: "within a reasonable period after the request is made (section 21T(3))",
            charge: "Any charge for giving access must not be excessive and must not apply to making the request (section 21T(6)).",
        },
        Lever::S21vCorrection => Law {
            title: "Correct my credit information",
            provision: "Privacy Act s 21V(1), (2)",
            deadline: Deadline::Days(30),
            deadline_text: "within 30 days from the day the request is made, or a longer period I agree to in writing (section 21V(2))",
            charge: "No charge may be made for the request or the correction (section 21V(5)).",
        },
        Lever::SpamUnsubscribe => Law {
            title: "Stop sending me commercial electronic messages",
            provision: "Spam Act 2003 s 18; Sch 2 cl 6",
            deadline: Deadline::BusinessDays(5),
            deadline_text: "at the end of the period of 5 business days beginning on the day this email is sent (Spam Act 2003, Schedule 2, clause 6(1)(e))",
            charge: "Using the unsubscribe address must not require a fee, an account or personal information other than the address itself (Spam Regulations 2021, section 7).",
        },
    }
}
