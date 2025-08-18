// 최소한의 Makepad 앱 - 정말 기본만 
use makepad_widgets::*;

live_design!{
    import makepad_widgets::base::*;
    import makepad_widgets::theme_desktop_dark::*;
    
    App = {{App}} {
        ui: <Window>{
            <View>{
                <Label>{text:"Hello!"}
            }
        }
    }
}

#[derive(Live, LiveHook)]
pub struct App {
    #[live] ui: WidgetRef
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
    }
}

impl MatchEvent for App {}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}

fn main() {
    println!("Minimal app starting...");
    app_main!(App);
    println!("Minimal app ended");
}



