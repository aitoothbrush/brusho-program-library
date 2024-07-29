pub use deposit_entry::*;
pub use lockup::*;
pub use registrar::*;
pub use voter::*;

mod deposit_entry;
mod lockup;
mod registrar;
mod voter;

use crate::vote_weight_record;

// Generate a VoteWeightRecord Anchor wrapper, owned by the current program.
// VoteWeightRecords are unique in that they are defined by the SPL governance
// program, but they are actually owned by this program.
vote_weight_record!(crate::ID);
