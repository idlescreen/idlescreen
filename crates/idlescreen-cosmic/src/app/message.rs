// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use cosmic::iced::window::Id;

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    /// libcosmic surface/tooltip plumbing (COSMIC applet pattern).
    Surface(cosmic::surface::Action),
    SubscriptionChannel,
    Refresh,
    UpdateConfig(crate::config::Config),
    ToggleIdleEnabled(bool),
    ActiveSaverSelected(String),
    ToggleDaemon(bool),
    ToggleFpsOverlay(bool),
    ToggleAdvanced,
    DecreaseTimeout,
    IncreaseTimeout,
    MiddleClick,
    TriggerPreview,
    OpenDashboard,
    ChangeRenderScale(f32),
}
