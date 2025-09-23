pub mod ui{
    use tui::{backend::Backend, widgets::{List, ListItem}, Frame};

    pub fn build_directory_list() -> List<'static>{
        let items = [ListItem::new("Ahke")];

        List::new(items)
    }


    pub fn build_ui<B: Backend>(f: &mut Frame<B>){

    }
}
