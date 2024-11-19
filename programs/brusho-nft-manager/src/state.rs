use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct Maker {
    pub realm: Pubkey,
    pub realm_authority: Pubkey,
    pub collection: Pubkey,
    pub merkle_tree: Pubkey,
    pub update_authority: Pubkey,
    pub issuing_authority: Pubkey,
    pub name: String,
    pub is_active: bool,
    pub bump: u8,
    pub collection_bump: u8,
}

#[macro_export]
macro_rules! maker_seeds {
    ( $maker:expr ) => {
        &[
            b"maker",
            $maker.realm.as_ref(),
            $maker.name.as_bytes(),
            &[$maker.bump],
        ]
    };
}

pub use maker_seeds;

#[account]
#[derive(Default)]
pub struct BrushNoToAsset {
    pub realm: Pubkey,
    pub asset: Pubkey,
    pub brush_no: String,
    pub bump: u8,
}


#[macro_export]
macro_rules! brush_no_to_asset_seeds {
  ( $brush_no_to_asset:expr ) => {
    &[
      "brush_no_to_asset".as_bytes(),
      $brush_no_to_asset.realm.as_ref(),
      $brush_no_to_asset.brush_no.as_bytes(),
      &[$brush_no_to_asset.bump],
    ]
  };
}

pub use brush_no_to_asset_seeds;
