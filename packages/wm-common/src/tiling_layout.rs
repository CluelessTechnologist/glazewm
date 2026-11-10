use std::str::FromStr;

use anyhow::bail;
use serde::{Deserialize, Serialize};

/// How the tiling children of a direction container (workspace or split
/// container) are laid out.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TilingLayout {
  /// Children are placed side by side along the tiling direction.
  #[default]
  Tiles,

  /// Children take up the full container below a tab bar, and only the
  /// active tab (most recently focused child) is shown.
  Tabbed,
}

impl TilingLayout {
  /// Gets the other layout.
  ///
  /// Example:
  /// ```
  /// # use wm_common::TilingLayout;
  /// let layout = TilingLayout::Tiles.toggled();
  /// assert_eq!(layout, TilingLayout::Tabbed);
  /// ```
  #[must_use]
  pub fn toggled(&self) -> Self {
    match self {
      Self::Tiles => Self::Tabbed,
      Self::Tabbed => Self::Tiles,
    }
  }
}

impl FromStr for TilingLayout {
  type Err = anyhow::Error;

  /// Parses a string into a tiling layout.
  ///
  /// Example:
  /// ```
  /// # use wm_common::TilingLayout;
  /// # use std::str::FromStr;
  /// let layout = TilingLayout::from_str("tabbed");
  /// assert_eq!(layout.unwrap(), TilingLayout::Tabbed);
  /// ```
  fn from_str(unparsed: &str) -> anyhow::Result<Self> {
    match unparsed {
      "tiles" => Ok(Self::Tiles),
      "tabbed" => Ok(Self::Tabbed),
      _ => bail!("Not a valid tiling layout: {}", unparsed),
    }
  }
}
