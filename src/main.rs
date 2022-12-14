#[macro_use]
extern crate penrose;

use penrose::{
    contrib::{
        extensions::Scratchpad,
        hooks::{DefaultWorkspace, LayoutSymbolAsRootName},
        layouts::paper,
    },
    core::{
        config::Config,
        helpers::index_selectors,
        hooks::Hook,
        layout::{bottom_stack, side_stack, Layout, LayoutConf},
        manager::WindowManager,
        ring::Selector,
        xconnection::{XConn, Xid},
    },
    logging_error_handler,
    xcb::{new_xcb_backed_window_manager, XcbConnection, XcbHooks},
    Backward, Forward, Less, More, Result,
};

use simplelog::{LevelFilter, SimpleLogger};
use tracing::info;

// Replace these with your preferred terminal and program launcher
const TERMINAL: &str = "alacritty";
const LAUNCHER: &str = "/home/connor/.config/rofi/scripts/launcher.sh";
const POWERMENU: &str = "/home/connor/.config/rofi/scripts/powermenu.sh";
const FOCUSED_BORDER: &str = "#cc241d";
const UNFOCUSED_BORDER: &str = "#3c3836";

// An example of a simple custom hook. In this case we are creating a NewClientHook which
// will be run eact time a new client program is spawned
struct MyClientHook {}
impl<X: XConn> Hook<X> for MyClientHook {
    fn new_client(&mut self, wm: &mut WindowManager<X>, id: Xid) -> Result<()> {
        let c = wm.client(&Selector::WinId(id)).unwrap();
        info!("new client with WM_CLASS='{}' WM_NAME='{}'", c.wm_class(), c.wm_name());
        Ok(())
    }
}


fn main() -> penrose::Result<()> {
    // Initialise the logger (use LevelFilter::Debug to enable debug logging)
    if let Err(e) = SimpleLogger::init(LevelFilter::Info, simplelog::Config::default()) {
        panic!("unable to set log level: {}", e);
    };

    // Created at startup. See keybindings below for how to access them
    let mut config_builder = Config::default().builder();
    config_builder
        .workspaces(vec!["1", "2", "3", "4", "5", "6", "7", "8", "9"])
        // Windows with a matching WM_CLASS will always float
        .floating_classes(vec!["dmenu", "dunst", "polybar"])
        // Client border colors are set based on X focus
        .focused_border(FOCUSED_BORDER)?
        .unfocused_border(UNFOCUSED_BORDER)?;

    // When specifying a layout, most of the time you will want LayoutConf::default() as
    // shown below, which will honour gap settings and will not be run on focus changes
    // (only when clients are added/removed). To customise when/how each layout is applied
    // you can create a LayoutConf instance with your desired properties enabled.
    let follow_focus_conf = LayoutConf {
        floating: false,
        gapless: true,
        follow_focus: true,
        allow_wrapping: false,
    };

    // Default number of clients in the main layout area
    let n_main = 1;

    // Default percentage of the screen to fill with the main area of the layout
    let ratio = 0.6;

    // Layouts to be used on each workspace. Currently all workspaces have the same set 
    // of Layouts available to them, though they track modifications to n_main and ratio
    // independently.
    config_builder.layouts(vec![
        Layout::new("[side]", LayoutConf::default(), side_stack, n_main, ratio),
        Layout::new("[botm]", LayoutConf::default(), bottom_stack, n_main, ratio),
        Layout::new("[papr]", follow_focus_conf, paper, n_main, ratio),
        Layout::floating("[----]"),
    ]);

    // Now build and validate the config
    let config = config_builder.build().unwrap();

    /* hooks
     *
     * penrose provides several hook points where you can run your own code as part of
     * WindowManager methods. This allows you to trigger custom code without having to use
     * a key binding to do so. See the hooks module in the docs for details of what hooks
     * are avaliable and when/how they will be called. Note that each class of hook will be
     * called in the order that they are defined. Hooks may maintain their own internal
     * state which they can use to modify their behaviour if desired.
     */

    // Scratchpad is an extension: it makes use of the same Hook points as the examples
    // below but additionally provides a 'toggle' method that can be bound to a key
    // combination in order to trigger the bound scratchpad client.
    let sp = Scratchpad::new(TERMINAL, 0.8, 0.8);

    let hooks: XcbHooks = vec![
        Box::new(MyClientHook {}),
        // Using a simple contrib hook that takes no config. By convention, contrib hooks 
        // have a 'new' method that returns a boxed instance of the hook with any
        // configuration performed so that it is ready to push onto the corresponding 
        // *_hooks vec.
        LayoutSymbolAsRootName::new(),
        // Here we are using a contrib hook that requires configuration to set up a default
        // workspace on workspace "9". This will set the layout and spawn the supplied
        // programs if we make workspace "9" active while it has no clients.
        DefaultWorkspace::new(
            "9",
            "[botm]",
            vec![TERMINAL, TERMINAL, "st"],
        ),
        sp.get_hook(),
    ];

    /* The gen_keybindings macro parses user friendly key binding definitions into X keycodes
     * and modifier masks. It uses the 'xmodmap' program to determine your current keymap
     * and create the bindings dynamically on startup. If this feels a little too magical
     * then you can alternatively construct a HashMap<KeyCode, FireAndForget> manually with
     * your chosen keybindings (see helpers.rs and data_types.rs for details).
     * FireAndForget functions do not need to make use of the mutable WindowManager
     * reference they are passed if it is not required: the run_external macro ignores the
     * WindowManager itself and instead spawns a new child process.
     */
    let key_bindings = gen_keybindings! {
        // Program launchers
        "M-semicolon" => run_external!(LAUNCHER);
        "M-Return" => run_external!(TERMINAL);
        "M-S-x" => run_external!(POWERMENU);

        // Exit Penrose (important to remember this one!)
        "M-A-C-Escape" => run_internal!(exit);

        "M-A-s" => run_internal!(detect_screens);

        // client management
        "M-j" => run_internal!(cycle_client, Forward);
        "M-k" => run_internal!(cycle_client, Backward);
        "M-S-j" => run_internal!(drag_client, Forward);
        "M-S-k" => run_internal!(drag_client, Backward);
        "M-S-f" => run_internal!(toggle_client_fullscreen, &Selector::Focused);
        "M-S-q" => run_internal!(kill_client);

        "M-C-1" => run_internal!(client_to_screen, &Selector::Index(0));
        "M-C-2" => run_internal!(client_to_screen, &Selector::Index(1));
        "M-C-3" => run_internal!(client_to_screen, &Selector::Index(2));

        // workspace management
        "M-Tab" => run_internal!(toggle_workspace);
        "M-A-period" => run_internal!(cycle_workspace, Forward);
        "M-A-comma" => run_internal!(cycle_workspace, Backward);

        // Layout management
        "M-grave" => run_internal!(cycle_layout, Forward);
        "M-S-grave" => run_internal!(cycle_layout, Backward);
        "M-A-Up" => run_internal!(update_max_main, More);
        "M-A-Down" => run_internal!(update_max_main, Less);
        "M-A-Right" => run_internal!(update_main_ratio, More);
        "M-A-Left" => run_internal!(update_main_ratio, Less);

        // scratchpad management
        "M-slash" => sp.toggle();

        // Each keybinding here will be templated in with the workspace index of each
        // workspace, allowing for common workspace actions to be bound at once.
        map: { "1", "2", "3", "4", "5", "6", "7", "8", "9" } to index_selectors(9) => {
            "M-{}" => focus_workspace (REF);
            "M-S-{}" => client_to_workspace (REF);
        };
    };


    // The underlying connection to the X server is handled as a trait: XConn. XcbConnection
    // is the reference implementation of this trait that uses the XCB library to
    // communicate with the X server. You are free to provide your own implementation if 
    // you wish, see xconnection.rs for details of the required methods and expected
    // behaviour and xcb/xconn.rs for the implementation of XcbConnection.
    //let conn = XcbConnection::new()?;

    // Create the WindowManager instance with the config we have built and a connection to
    // the X server. Before calling grab_keys_and_run, it is possible to run additional
    // start-up actions such as configuring initial WindowManager state, running custom
    // code / hooks or spawning external processes such as a start-up script.
    //let mut wm = WindowManager::new(config, conn, hooks, logging_error_handler());
    //wm.init()?;

    // NOTE: If you are using the default XCB backend provided in the penrose xcb module,
    //       then the construction of the XcbConnection and resulting WindowManager can be
    //       done using the new_xcb_backed_window_manager helper function like so:
    //
    // let mut wm = new_xcb_backed_window_manager(config)?;
    let mut wm = new_xcb_backed_window_manager(config, hooks, logging_error_handler())?;

    // grab_keys_and_run will start listening to events from the X server and drop into the
    // main event loop. From this point on, program control passes to the WindowManager so
    // make sure that any logic you wish to run is done before here!
    wm.grab_keys_and_run(key_bindings, map!{})
}
