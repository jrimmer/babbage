-- Config for a throwaway nested Hyprland used to run the editor off-screen.
-- HEADLESS-2 is created at runtime with `hyprctl output create headless`;
-- it renders on its own 60 Hz timer, so clients get real frame callbacks
-- while nothing appears on the user's desktop.
hl.monitor({ output = "WAYLAND-1", mode = "1280x800", position = "0x0", scale = 1 })
hl.monitor({ output = "HEADLESS-2", mode = "2560x1440@60", position = "3000x0", scale = 1.25 })
hl.monitor({ output = "", mode = "preferred", position = "auto", scale = 1 })
hl.workspace_rule({ workspace = "2", monitor = "HEADLESS-2", default = true, persistent = true })
hl.config({ misc = { disable_hyprland_logo = true, disable_splash_rendering = true } })
-- Screenshots show the app's own pixels: no gaps, borders, dimming or blur.
-- The one window tiles to fill HEADLESS-2, as a lone window does for a user.
-- Animations are off because the window-open fade and pop-in last longer
-- than a check takes to connect, so an early grim caught the window shrunk
-- and blended with the wallpaper colour.
hl.config({
  general = { gaps_in = 0, gaps_out = 0, border_size = 0 },
  decoration = { rounding = 0, dim_inactive = false, active_opacity = 1.0, inactive_opacity = 1.0 },
  animations = { enabled = false },
})
