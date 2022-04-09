mod state;

use std::fmt::Debug;

use cgmath::num_traits::clamp;
use derive_new::new;
use cgmath::{ Vector4, Vector3, Vector2 };
use graphics::Renderer;
use vulkano::pipeline::shader::EntryPointAbstract;

use crate::graphics::{ Color, Transform };
use crate::input::UserEvent;
use crate::map::{ Object, LightSource, SoundSource, EffectSource, Particle };
use crate::map::model::Node;

pub use self::state::StateProvider;

pub enum HoverInformation {
    Element(usize, usize),
    Missed,
}

impl HoverInformation {
    
    pub fn to_element_identifier(self) -> usize {
        match self {
            HoverInformation::Element(_window_identifier, element_identifier) => element_identifier,
            _other => usize::MAX,
        }
    }
}

pub struct Interface {
    theme: Theme,
    windows: Vec<Box<dyn Window>>,
    creation_counter: CreationCounter,
}

impl Interface {

    pub fn new(/*screen_size: Size*/) -> Self {

        let screen_size = Size::new(100.0, Some(100.0));
        let mut creation_counter = CreationCounter::new();
        let mut windows = vec![renderer_settings_window(&mut creation_counter, screen_size)];

        windows.iter_mut().for_each(|window| window.validate_size(screen_size));
        windows.iter_mut().for_each(|window| window.update(screen_size));

        let theme = Theme {
            window_background_color: Color::new(50, 50, 50),
            window_text_color: Color::new(200, 100, 100),
            button_background_color: Color::new(100, 100, 100),
            button_text_color: Color::new(230, 230, 230),
        };

        return Self {
            windows,
            creation_counter,
            theme,
        };
    }

    pub fn hovered_element(&self, mouse_position: Vector2<f32>) -> HoverInformation {

        for (window_index, window) in self.windows.iter().enumerate() {
            if let Some(element_index) = window.hovered_element(mouse_position) {
                return HoverInformation::Element(window_index, element_index);
            }
        }
        
        return HoverInformation::Missed;
    }

    pub fn left_click(&mut self, window_index: usize, element_index: usize, state_provider: &mut StateProvider) -> Option<UserEvent> {

        let mut force_update = false;
        let event = self.windows[window_index].left_click(element_index, state_provider, &mut force_update);
        
        if force_update {
            let screen_size = Size::new(100.0, Some(100.0));
            self.windows[window_index].update(screen_size);
        }

        return event;
    }

    pub fn move_window(&mut self, index: usize, offset: Vector2<f32>) {
        self.windows[index].offset(offset);
    }

    pub fn resize_window(&mut self, index: usize, growth: Vector2<f32>) {
        let screen_size = Size::new(100.0, Some(100.0));
        self.windows[index].resize(screen_size, growth);
    }

    pub fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, hovered_element: usize) {
        self.windows.iter().for_each(|window| window.render(renderer, state_provider, &self.theme, hovered_element));
    }

    fn open_new_window(&mut self, mut window: Box<dyn Window + 'static>) {
        let screen_size = Size::new(100.0, Some(100.0));
        window.validate_size(screen_size);
        window.update(screen_size);
        self.windows.push(window);
    }

    pub fn close_window(&mut self, window_identifier: usize) {
        self.windows.retain(|window| !window.identifier_matches(window_identifier));
    }

    pub fn open_object_window(&mut self, object: Object, index: usize) {
        let screen_size = Size::new(100.0, Some(100.0));
        let window = object_window(&mut self.creation_counter, screen_size, object, index);
        self.open_new_window(window);
    }

    pub fn open_light_source_window(&mut self, light_source: LightSource, index: usize) {
        let screen_size = Size::new(100.0, Some(100.0));
        let window = light_source_window(&mut self.creation_counter, screen_size, light_source, index);
        self.open_new_window(window);
    }

    pub fn open_sound_source_window(&mut self, sound_source: SoundSource, index: usize) {
        let screen_size = Size::new(100.0, Some(100.0));
        let window = sound_source_window(&mut self.creation_counter, screen_size, sound_source, index);
        self.open_new_window(window);
    }

    pub fn open_effect_source_window(&mut self, effect_source: EffectSource, index: usize) {
        let screen_size = Size::new(100.0, Some(100.0));
        let window = effect_source_window(&mut self.creation_counter, screen_size, effect_source, index);
        self.open_new_window(window);
    }

    pub fn open_particle_window(&mut self, particle: Particle, index: usize, particle_index: usize) {
        let screen_size = Size::new(100.0, Some(100.0));
        let window = particle_window(&mut self.creation_counter, screen_size, particle, index, particle_index);
        self.open_new_window(window);
    }
}







pub struct Theme {
    window_background_color: Color,
    window_text_color: Color,
    button_background_color: Color,
    button_text_color: Color,
}


pub enum ClickAction {
    Handeled,
    Event(UserEvent),
    None,
}

impl ClickAction {
    
    pub fn is_none(&self) -> bool {
        matches!(self, ClickAction::None)
    }
}

const THRESHHOLD: f32 = 0.0001; 

pub struct PlacementResolver {
    avalible_space: Size,
    base_position: Vector2<f32>,
    horizontal_accumulator: f32,
    vertical_offset: f32,
    total_height: f32,
    gaps: Vector2<f32>,
}

impl PlacementResolver {
    
    pub fn new(mut avalible_space: Size, base_position: Vector2<f32>, border: Vector2<f32>, gaps: Vector2<f32>) -> Self {

        avalible_space.width -= border.x * 2.0;
        avalible_space.height = avalible_space.height.map(|height| height - border.y * 2.0);

        let base_position = base_position + border;
        let horizontal_accumulator = 0.0;
        let vertical_offset = 0.0;
        let total_height = 0.0;

        Self { avalible_space, base_position, horizontal_accumulator, total_height, vertical_offset, gaps }
    }

    pub fn derive(&self, offset: Vector2<f32>, border: Vector2<f32>) -> Self {

        let mut avalible_space = self.avalible_space.clone();
        avalible_space.width -= offset.x + border.x * 2.0;
        avalible_space.height = avalible_space.height.map(|height| height - border.y * 2.0);

        let base_position = offset + border;
        let horizontal_accumulator = 0.0;
        let vertical_offset = 0.0;
        let total_height = 0.0;
        let gaps = self.gaps;

        Self { avalible_space, base_position, horizontal_accumulator, total_height, vertical_offset, gaps }
    }

    pub fn set_gaps(&mut self, gaps: Vector2<f32>) {
        self.gaps = gaps;
    }

    pub fn get_avalible(&self) -> Size {
        self.avalible_space
    }

    pub fn newline(&mut self) {
        self.total_height += self.vertical_offset + self.gaps.y;
        self.base_position.y += self.vertical_offset + self.gaps.y;
        self.horizontal_accumulator = 0.0;
        self.vertical_offset = 0.0;
    }

    pub fn register_height(&mut self, height: f32) {
        self.vertical_offset = f32::max(self.vertical_offset, height);
    }

    pub fn allocate(&mut self, size_constraint: &SizeConstraint) -> (Size, Vector2<f32>) {
        
        let mut size = size_constraint.resolve(self.avalible_space);
        let remaining_width = self.avalible_space.width - self.horizontal_accumulator;

        if remaining_width < size.width - THRESHHOLD {
            self.newline();
        }
        
        let position = Vector2::new(self.base_position.x + self.horizontal_accumulator + self.gaps.x, self.base_position.y);

        self.horizontal_accumulator += size.width;

        if let Some(height) = size.height {
            self.register_height(height);
        }

        size.width -= self.gaps.x;
        return (size, position);
    }

    pub fn allocate_right(&mut self, size_constraint: &SizeConstraint) -> (Size, Vector2<f32>) {
        
        let mut size = size_constraint.resolve(self.avalible_space);
        let remaining_width = self.avalible_space.width - self.horizontal_accumulator;

        if remaining_width < size.width - THRESHHOLD {
            self.newline();
        }
        
        let position = Vector2::new(self.base_position.x + (self.avalible_space.width - size.width), self.base_position.y);

        self.horizontal_accumulator += remaining_width;

        if let Some(height) = size.height {
            self.register_height(height);
        }

        size.width -= self.gaps.x;
        return (size, position);
    }

    pub fn final_height(self) -> f32 {
        self.total_height + self.vertical_offset
    }
}

#[derive(new)]
pub struct CreationCounter {
    #[new(default)]
    element_counter: usize,
}

impl CreationCounter {
    
   pub fn allocate(&mut self) -> usize {
       let identifier = self.element_counter;
       self.element_counter += 1;
       return identifier;
   }
}

pub trait Element {

    fn update(&mut self, placement_resolver: &mut PlacementResolver);

    fn hovered_element(&self, mouse_position: Vector2<f32>) -> Option<usize>;
    
    fn try_left_click(&mut self, hovered_element: usize, state_provider: &mut StateProvider, force_update: &mut bool) -> ClickAction;

    fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, theme: &Theme, parent_position: Vector2<f32>, hovered_element: usize);  
}

#[derive(Copy, Clone)]
pub enum Dimension {
    Relative(f32, f32),
    Absolute(f32),
}

impl Dimension {
    
    pub fn resolve(&self, avalible: Option<f32>) -> f32 {
        match *self {
            Dimension::Relative(precentage, minimum) => f32::max(minimum, avalible.expect("trying to get a relative height from a flexible component") / 100.0 * precentage),
            Dimension::Absolute(value) => value,
        }
    }
}

#[derive(Copy, Clone, new)]
pub struct SizeConstraint {
    width: Dimension,
    height: Option<Dimension>,
}

impl SizeConstraint {
    
    pub fn resolve(&self, avalible: Size) -> Size {

        let width = self.width.resolve(Some(avalible.width));
        let height = self.height.map(|constraint| constraint.resolve(avalible.height));

        return Size::new(width, height);
    }
}

#[derive(Copy, Clone, new)]
pub struct Size {
    width: f32,
    height: Option<f32>,
}

impl Size {
    
    pub fn unwrap(self) -> Vector2<f32> {
        let x = self.width;
        let y = self.height.expect("element cannot have flexible height");
        return Vector2::new(x, y);
    }
    
    pub fn unwrap_or(self, height: f32) -> Vector2<f32> {
        let x = self.width;
        let y = self.height.unwrap_or(height);
        return Vector2::new(x, y);
    }
}

pub struct StateButton {
    text: &'static str,
    action: Box<dyn FnMut(&mut StateProvider)>,
    selector: Box<dyn Fn(&StateProvider) -> bool>,
    size_constraint: SizeConstraint,
    cached_size: Vector2<f32>,
    cached_position: Vector2<f32>,
    identifier: usize,
}

impl StateButton {
    
    pub const DEFAULT_SIZE: SizeConstraint = SizeConstraint { width: Dimension::Relative(100.0, 50.0), height: Some(Dimension::Absolute(14.0)) };

    pub fn new(creation_counter: &mut CreationCounter, text: &'static str, action: Box<dyn FnMut(&mut StateProvider)>, selector: Box<dyn Fn(&StateProvider) -> bool>, size_constraint: SizeConstraint) -> Self {

        Self {
            text,
            action,
            selector,
            size_constraint,
            cached_size: Vector2::new(0.0, 0.0),
            cached_position: Vector2::new(0.0, 0.0),
            identifier: creation_counter.allocate(),
        }
    }
}

impl Element for StateButton {

    fn update(&mut self, placement_resolver: &mut PlacementResolver) {
        let (size, position) = placement_resolver.allocate(&self.size_constraint);
        self.cached_size = size.unwrap();
        self.cached_position = position;
    }

    fn hovered_element(&self, mouse_position: Vector2<f32>) -> Option<usize> {

        let absolute_position = mouse_position - self.cached_position;
        
        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.cached_size.x && absolute_position.y <= self.cached_size.y {
            return Some(self.identifier);
        }

        return None;
    }
    
    fn try_left_click(&mut self, hovered_element: usize, state_provider: &mut StateProvider, _force_update: &mut bool) -> ClickAction {
        if self.identifier == hovered_element {
            (self.action)(state_provider);
            return ClickAction::Handeled;
        }
        return ClickAction::None;
    }
    
    fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, theme: &Theme, parent_position: Vector2<f32>, hovered_element: usize) {
        let absolute_position = parent_position + self.cached_position;

        match hovered_element == self.identifier {
            true => renderer.render_rectangle(absolute_position, self.cached_size, Vector4::new(0.0, 0.0, 0.0, 0.0), Color::new(180, 180, 180)),
            false => renderer.render_rectangle(absolute_position, self.cached_size, Vector4::new(0.0, 0.0, 0.0, 0.0), theme.button_background_color),
        }

        renderer.render_checkbox(absolute_position + Vector2::new(5.0, 2.0), Vector2::new(self.cached_size.y - 4.0, self.cached_size.y - 4.0), theme.button_text_color, (self.selector)(state_provider));
        renderer.render_text(&self.text, absolute_position + Vector2::new(20.0, 0.0), theme.button_text_color, 14.0);
    }
}

pub struct CloseButton {
    window_index: usize,
    size_constraint: SizeConstraint,
    cached_size: Vector2<f32>,
    cached_position: Vector2<f32>,
    identifier: usize,
}

impl CloseButton {
    
    pub const DEFAULT_SIZE: SizeConstraint = SizeConstraint { width: Dimension::Absolute(25.0), height: Some(Dimension::Absolute(14.0)) };

    pub fn new(creation_counter: &mut CreationCounter, window_index: usize, size_constraint: SizeConstraint) -> Self {

        Self {
            window_index,
            size_constraint,
            cached_size: Vector2::new(0.0, 0.0),
            cached_position: Vector2::new(0.0, 0.0),
            identifier: creation_counter.allocate(),
        }
    }
}

impl Element for CloseButton {

    fn update(&mut self, placement_resolver: &mut PlacementResolver) {
        let (size, position) = placement_resolver.allocate_right(&self.size_constraint);
        self.cached_size = size.unwrap();
        self.cached_position = position;
    }

    fn hovered_element(&self, mouse_position: Vector2<f32>) -> Option<usize> {

        let absolute_position = mouse_position - self.cached_position;
        
        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.cached_size.x && absolute_position.y <= self.cached_size.y {
            return Some(self.identifier);
        }

        return None;
    }
    
    fn try_left_click(&mut self, hovered_element: usize, _state_provider: &mut StateProvider, _force_update: &mut bool) -> ClickAction {
        if self.identifier == hovered_element {
            return ClickAction::Event(UserEvent::CloseWindow(self.window_index));
        }
        return ClickAction::None;
    }
    
    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, theme: &Theme, parent_position: Vector2<f32>, hovered_element: usize) {
        let absolute_position = parent_position + self.cached_position;

        match hovered_element == self.identifier {
            true => renderer.render_rectangle(absolute_position, self.cached_size, Vector4::new(0.0, 0.0, 0.0, 0.0), Color::new(200, 100, 100)),
            false => renderer.render_rectangle(absolute_position, self.cached_size, Vector4::new(0.0, 0.0, 0.0, 0.0), Color::new(150, 100, 100)),
        }

        renderer.render_text("X", absolute_position + Vector2::new(5.0, 0.0), Color::new(210, 150, 150), 14.0);
    }
}

pub struct Expandable {

    display: String,

    expanded: bool,

    open_size_constraint: SizeConstraint,

    closed_size_constraint: SizeConstraint,

    cached_size: Vector2<f32>,

    cached_position: Vector2<f32>,

    identifier: usize,

    elements: Vec<Box<dyn Element>>,
}

impl Expandable {
     
    fn new(creation_counter: &mut CreationCounter, display: String, elements: Vec<Box<dyn Element>>) -> Self {

        Self {
            display,
            expanded: true,
            open_size_constraint: SizeConstraint::new(Dimension::Relative(100.0, 100.0), None),
            closed_size_constraint: SizeConstraint::new(Dimension::Relative(100.0, 100.0), Some(Dimension::Absolute(18.0))),
            cached_size: Vector2::new(0.0, 0.0),
            cached_position: Vector2::new(0.0, 0.0),
            identifier: creation_counter.allocate(),
            elements,
        }
    }
}

impl Element for Expandable {

    fn update(&mut self, placement_resolver: &mut PlacementResolver) {

        let closed_size = self.closed_size_constraint.resolve(placement_resolver.get_avalible()).unwrap();

        let (mut size, position) = match self.expanded {
            true => placement_resolver.allocate(&self.open_size_constraint),
            false => placement_resolver.allocate(&self.closed_size_constraint),
        };

        if self.expanded {
            let mut inner_placement_resolver = placement_resolver.derive(Vector2::new(5.0, closed_size.y), Vector2::new(5.0, 0.0));
            inner_placement_resolver.set_gaps(Vector2::new(2.0, 2.0));

            self.elements.iter_mut().for_each(|element| element.update(&mut inner_placement_resolver));

            if self.open_size_constraint.height.is_none() {
                let final_height = closed_size.y + 5.0 + inner_placement_resolver.final_height();
                size.height = Some(final_height);
                placement_resolver.register_height(final_height);
            }
        }

        self.cached_size = size.unwrap();
        self.cached_position = position;
    }

    fn hovered_element(&self, mouse_position: Vector2<f32>) -> Option<usize> {

        let absolute_position = mouse_position - self.cached_position;
        
        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.cached_size.x && absolute_position.y <= self.cached_size.y {

            if self.expanded {
                for element in &self.elements {
                    if let Some(identifier) = element.hovered_element(absolute_position) {
                        return Some(identifier);
                    }
                }
            } 

            return Some(self.identifier);
        }

        return None;
    }
    
    fn try_left_click(&mut self, hovered_element: usize, state_provider: &mut StateProvider, force_update: &mut bool) -> ClickAction {
        
        if self.identifier == hovered_element {
            self.expanded = !self.expanded;
            *force_update = true;
            return ClickAction::Handeled;
        } else if self.expanded {
            for element in &mut self.elements {
                let click_action = element.try_left_click(hovered_element, state_provider, force_update);

                if !click_action.is_none() { 
                    return click_action;
                }
            }
        }

        return ClickAction::None;
    }

    fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, theme: &Theme, parent_position: Vector2<f32>, hovered_element: usize) {
        let absolute_position = parent_position + self.cached_position;

        renderer.render_rectangle(absolute_position, self.cached_size, Vector4::new(0.0, 0.0, 0.0, 0.0), Color::new(70, 70, 70));

        match hovered_element == self.identifier {
            true => renderer.render_text(&self.display, absolute_position + Vector2::new(5.0, 0.0), Color::new(250, 250, 250), 14.0),
            false => renderer.render_text(&self.display, absolute_position + Vector2::new(5.0, 0.0), Color::new(200, 200, 200), 14.0),
        }

        if self.expanded {
            self.elements.iter().for_each(|element| element.render(renderer, state_provider, theme, absolute_position, hovered_element));
        }
    }
}

pub trait Window {

    fn identifier_matches(&self, window_identifier: usize) -> bool;

    fn update(&mut self, avalible_space: Size);

    fn hovered_element(&self, mouse_position: Vector2<f32>) -> Option<usize>;
   
    fn left_click(&mut self, hovered_element: usize, state_provider: &mut StateProvider, force_update: &mut bool) -> Option<UserEvent>;

    fn offset(&mut self, offset: Vector2<f32>);

    fn resize(&mut self, avalible_space: Size, growth: Vector2<f32>);
    
    fn validate_size(&mut self, avalible_space: Size);

    fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, theme: &Theme, hovered_element: usize);
}

pub struct FramedWindow {

    title: String,

    position: Vector2<f32>,

    size_constraint: SizeConstraint,

    minimum_size: SizeConstraint,
    
    maximum_size: SizeConstraint,
    
    cached_size: Vector2<f32>,

    elements: Vec<Box<dyn Element>>,

    identifier: usize,
}

impl FramedWindow {
    
    fn new(creation_counter: &mut CreationCounter, title: String, mut elements: Vec<Box<dyn Element>>, avalible_space: Size) -> Self {

        let identifier = creation_counter.allocate();
        let close_button = Box::new(CloseButton::new(creation_counter, identifier, CloseButton::DEFAULT_SIZE)) as _;
        elements.insert(0, close_button);

        let position = Vector2::new(100.0, 300.0);
        let size_constraint = SizeConstraint::new(Dimension::Absolute(300.0), None);
        let minimum_size = SizeConstraint::new(Dimension::Absolute(200.0), Some(Dimension::Absolute(80.0)));
        let maximum_size = SizeConstraint::new(Dimension::Absolute(400.0), None);
        let cached_size = size_constraint.resolve(avalible_space).unwrap_or(0.0);

        Self {
            title,
            position,
            size_constraint,
            minimum_size,
            maximum_size,
            cached_size,
            elements,
            identifier,
        }
    }
}

impl Window for FramedWindow {

    fn identifier_matches(&self, window_identifier: usize) -> bool {
        window_identifier == self.identifier
    }

    fn update(&mut self, avalible_space: Size) {

        let mut placement_resolver = PlacementResolver::new(Size::new(self.cached_size.x, self.size_constraint.height.map(|_| self.cached_size.y)), Vector2::new(0.0, 0.0), Vector2::new(10.0, 5.0), Vector2::new(2.0, 5.0));
        
        self.elements.iter_mut().for_each(|element| element.update(&mut placement_resolver));

        if self.size_constraint.height.is_none() {
            let final_height = 7.0 + placement_resolver.final_height();
            self.cached_size.y = final_height;
            self.validate_size(avalible_space);
        }
    }

    fn hovered_element(&self, mouse_position: Vector2<f32>) -> Option<usize> {

        let absolute_position = mouse_position - self.position;

        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.cached_size.x && absolute_position.y <= self.cached_size.y {            

            for element in &self.elements {
                if let Some(identifier) = element.hovered_element(absolute_position) {
                    return Some(identifier);
                }
            }

            return Some(usize::MAX);
        }

        return None;
    }

    fn left_click(&mut self, hovered_element: usize, state_provider: &mut StateProvider, force_update: &mut bool) -> Option<UserEvent> {

        for element in &mut self.elements {
            let click_action = element.try_left_click(hovered_element, state_provider, force_update);

            if let ClickAction::Event(event) = click_action {
                return Some(event);
            }
        }

        return None;
    }

    fn offset(&mut self, offset: Vector2<f32>) {
        self.position += offset;
    }

    fn resize(&mut self, avalible_space: Size, growth: Vector2<f32>) {
        self.cached_size += growth;
        self.validate_size(avalible_space);
        self.update(avalible_space);
    }

    fn validate_size(&mut self, avalible_space: Size) {
        let minimum_size = self.minimum_size.resolve(avalible_space);
        let maximum_size = self.maximum_size.resolve(avalible_space);

        self.cached_size.x = clamp(self.cached_size.x, minimum_size.width, maximum_size.width);
        
        if let Some(minimum_height) = minimum_size.height {
            self.cached_size.y = f32::max(self.cached_size.y, minimum_height); 
        }

        if let Some(maximum_height) = maximum_size.height {
            self.cached_size.y = f32::min(self.cached_size.y, maximum_height); 
        }
    }

    fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, theme: &Theme, hovered_element: usize) {
        renderer.render_rectangle(self.position, self.cached_size, Vector4::new(0.0, 0.0, 0.0, 0.0), theme.window_background_color);
        renderer.render_text(&self.title, self.position + Vector2::new(5.0, 4.0), theme.window_text_color, 14.0);
        self.elements.iter().for_each(|element| element.render(renderer, state_provider, theme, self.position, hovered_element));
    }
}

macro_rules! render_state_button {
    ($creation_counter:expr, $display:expr, $action:ident, $selector:ident, $size_constraint:expr) => {
        {
            let action = Box::new(|state_provider: &mut StateProvider| state_provider.render_settings.$action());
            let selector = Box::new(|state_provider: &StateProvider| state_provider.render_settings.$selector);
            Box::new(StateButton::new($creation_counter, $display, action, selector, $size_constraint)) as _
        }
    };
}

fn map_expandable(creation_counter: &mut CreationCounter) -> Box<dyn Element + 'static> {

    let buttons = vec![
        render_state_button!(creation_counter, "show fps", toggle_show_frames_per_second, show_frames_per_second, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "show map", toggle_show_map, show_map, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "show objects", toggle_show_objects, show_objects, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "show entities", toggle_show_entities, show_entities, StateButton::DEFAULT_SIZE),
    ];
    
    Box::new(Expandable::new(creation_counter, "map".to_string(), buttons)) as _
}

fn lighting_expandable(creation_counter: &mut CreationCounter) -> Box<dyn Element + 'static> {

    let buttons = vec![
        render_state_button!(creation_counter, "ambient light", toggle_show_ambient_light, show_ambient_light, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "directional light", toggle_show_directional_light, show_directional_light, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "point lights", toggle_show_point_lights, show_point_lights, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "particle lights", toggle_show_particle_lights, show_particle_lights, StateButton::DEFAULT_SIZE),
    ];
    
    Box::new(Expandable::new(creation_counter, "lighting".to_string(), buttons)) as _
}

fn markers_expandable(creation_counter: &mut CreationCounter) -> Box<dyn Element + 'static> {

    let buttons = vec![
        render_state_button!(creation_counter, "object markers", toggle_show_object_markers, show_object_markers, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "light markers", toggle_show_light_markers, show_light_markers, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "sound markers", toggle_show_sound_markers, show_sound_markers, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "effect markers", toggle_show_effect_markers, show_effect_markers, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "particle markers", toggle_show_particle_markers, show_particle_markers, StateButton::DEFAULT_SIZE),
    ];
    
    Box::new(Expandable::new(creation_counter, "markers".to_string(), buttons)) as _
}

fn grid_expandable(creation_counter: &mut CreationCounter) -> Box<dyn Element + 'static> {

    let buttons = vec![
        render_state_button!(creation_counter, "map tiles", toggle_show_map_tiles, show_map_tiles, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "pathing", toggle_show_pathing, show_pathing, StateButton::DEFAULT_SIZE),
    ];
    
    Box::new(Expandable::new(creation_counter, "grid".to_string(), buttons)) as _
}

fn buffers_expandable(creation_counter: &mut CreationCounter) -> Box<dyn Element + 'static> {

    let buttons = vec![
        render_state_button!(creation_counter, "diffuse buffer", toggle_show_diffuse_buffer, show_diffuse_buffer, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "normal buffer", toggle_show_normal_buffer, show_normal_buffer, StateButton::DEFAULT_SIZE),
        render_state_button!(creation_counter, "depth buffer", toggle_show_depth_buffer, show_depth_buffer, StateButton::DEFAULT_SIZE),
    ];
    
    Box::new(Expandable::new(creation_counter, "buffers".to_string(), buttons)) as _
}

pub fn renderer_settings_window(creation_counter: &mut CreationCounter, avalible_space: Size) -> Box<dyn Window + 'static> {

    let elements = vec![
        render_state_button!(creation_counter, "debug camera", toggle_use_debug_camera, use_debug_camera, StateButton::DEFAULT_SIZE),
        map_expandable(creation_counter),
        lighting_expandable(creation_counter),
        markers_expandable(creation_counter),
        grid_expandable(creation_counter),
        buffers_expandable(creation_counter),
    ];

    Box::new(FramedWindow::new(creation_counter, "render settings".to_string(), elements, avalible_space)) as _
}












pub struct ColorPreviewField {
    color: Color,
    size_constraint: SizeConstraint,
    cached_size: Vector2<f32>,
    cached_position: Vector2<f32>,
}

impl ColorPreviewField {
    
    pub const DEFAULT_SIZE: SizeConstraint = SizeConstraint { width: Dimension::Relative(100.0, 50.0), height: Some(Dimension::Absolute(14.0)) };

    pub fn new(color: Color, size_constraint: SizeConstraint) -> Self {

        Self {
            color,
            size_constraint,
            cached_size: Vector2::new(0.0, 0.0),
            cached_position: Vector2::new(0.0, 0.0),
        }
    }
}

impl Element for ColorPreviewField {

    fn update(&mut self, placement_resolver: &mut PlacementResolver) {
        let (size, position) = placement_resolver.allocate(&self.size_constraint);
        self.cached_size = size.unwrap();
        self.cached_position = position;
    }

    fn hovered_element(&self, _mouse_position: Vector2<f32>) -> Option<usize> {
        return None;
    }
    
    fn try_left_click(&mut self, _hovered_element: usize, _state_provider: &mut StateProvider, _force_update: &mut bool) -> ClickAction {
        return ClickAction::None;
    }
    
    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, _theme: &Theme, parent_position: Vector2<f32>, _hovered_element: usize) {
        let absolute_position = parent_position + self.cached_position;

        renderer.render_rectangle(absolute_position, self.cached_size, Vector4::new(0.0, 0.0, 0.0, 0.0), self.color);
    }
}

pub struct StaticStringField {
    text: String,
    size_constraint: SizeConstraint,
    cached_size: Vector2<f32>,
    cached_position: Vector2<f32>,
}

impl StaticStringField {
    
    pub const DEFAULT_SIZE: SizeConstraint = SizeConstraint { width: Dimension::Relative(100.0, 50.0), height: Some(Dimension::Absolute(14.0)) };

    pub fn new(text: String, size_constraint: SizeConstraint) -> Self {

        Self {
            text,
            size_constraint,
            cached_size: Vector2::new(0.0, 0.0),
            cached_position: Vector2::new(0.0, 0.0),
        }
    }
}

impl Element for StaticStringField {

    fn update(&mut self, placement_resolver: &mut PlacementResolver) {
        let (size, position) = placement_resolver.allocate(&self.size_constraint);
        self.cached_size = size.unwrap();
        self.cached_position = position;
    }

    fn hovered_element(&self, _mouse_position: Vector2<f32>) -> Option<usize> {
        return None;
    }
    
    fn try_left_click(&mut self, _hovered_element: usize, _state_provider: &mut StateProvider, _force_update: &mut bool) -> ClickAction {
        return ClickAction::None;
    }
    
    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, theme: &Theme, parent_position: Vector2<f32>, _hovered_element: usize) {
        let absolute_position = parent_position + self.cached_position;

        renderer.render_rectangle(absolute_position, self.cached_size, Vector4::new(0.0, 0.0, 0.0, 0.0), theme.button_background_color);
        renderer.render_text(&self.text, absolute_position + Vector2::new(5.0, 0.0), theme.button_text_color, 14.0);
    }
}

pub fn color_field(creation_counter: &mut CreationCounter, display: String, color: Color) -> Box<dyn Element + 'static> {

    let fields = vec![ 
        Box::new(ColorPreviewField::new(color, ColorPreviewField::DEFAULT_SIZE)) as _,
        Box::new(StaticStringField::new(format!("red: {}", color.red), StaticStringField::DEFAULT_SIZE)) as _,
        Box::new(StaticStringField::new(format!("green: {}", color.green), StaticStringField::DEFAULT_SIZE)) as _,
        Box::new(StaticStringField::new(format!("blue: {}", color.blue), StaticStringField::DEFAULT_SIZE)) as _,
    ];
    
    Box::new(Expandable::new(creation_counter, display, fields)) as _
}

pub fn vector3_field<T: Debug>(creation_counter: &mut CreationCounter, display: String, vector: Vector3<T>) -> Box<dyn Element + 'static> {

    let fields = vec![
        Box::new(StaticStringField::new(format!("x: {:?}", vector.x), StaticStringField::DEFAULT_SIZE)) as _,
        Box::new(StaticStringField::new(format!("y: {:?}", vector.y), StaticStringField::DEFAULT_SIZE)) as _,
        Box::new(StaticStringField::new(format!("z: {:?}", vector.z), StaticStringField::DEFAULT_SIZE)) as _,
    ];
    
    Box::new(Expandable::new(creation_counter, display, fields)) as _
}

pub fn transform_expandable(creation_counter: &mut CreationCounter, transform: Transform) -> Box<dyn Element + 'static> {

    let fields = vec![
        vector3_field(creation_counter, "position".to_string(), transform.position),
        vector3_field(creation_counter, "rotation".to_string(), transform.rotation),
        vector3_field(creation_counter, "scale".to_string(), transform.scale),
    ];
    
    Box::new(Expandable::new(creation_counter, "transform".to_string(), fields)) as _
}

fn create_node_interface(creation_counter: &mut CreationCounter, node: &Node, display: String) -> Box<dyn Element + 'static> {

    let mut elements = vec![
        Box::new(StaticStringField::new(format!("name: {}", node.name.clone()), StaticStringField::DEFAULT_SIZE)) as _,
        Box::new(StaticStringField::new(format!("parent name: {}", node.parent_name.clone().unwrap_or("none".to_string())), StaticStringField::DEFAULT_SIZE)) as _,
    ];

    if !node.child_nodes.is_empty() {
        let child_nodes = node.child_nodes.iter()
            .enumerate()
            .map(|(index, node)| create_node_interface(creation_counter, node, index.to_string()))
            .collect();
        let expandable = Box::new(Expandable::new(creation_counter, "child nodes".to_string(), child_nodes)) as _;
        elements.push(expandable);
    }
    
    Box::new(Expandable::new(creation_counter, display, elements)) as _
}

pub fn object_window(creation_counter: &mut CreationCounter, avalible_space: Size, object: Object, index: usize) -> Box<dyn Window + 'static> {

    let elements = vec![ 
        create_node_interface(creation_counter, &object.model.root_node, "model".to_string()),
        transform_expandable(creation_counter, object.transform),
    ];

    Box::new(FramedWindow::new(creation_counter, format!("object #{}", index), elements, avalible_space)) as _
}

pub fn light_source_window(creation_counter: &mut CreationCounter, avalible_space: Size, light_source: LightSource, index: usize) -> Box<dyn Window + 'static> {

    let elements = vec![ 
        vector3_field(creation_counter, "position".to_string(), light_source.position),
        color_field(creation_counter, "color".to_string(), light_source.color), 
        Box::new(StaticStringField::new(format!("range: {}", light_source.range), StaticStringField::DEFAULT_SIZE)) as _,
    ];

    Box::new(FramedWindow::new(creation_counter, format!("light source #{}", index), elements, avalible_space)) as _
}

pub fn sound_source_window(creation_counter: &mut CreationCounter, avalible_space: Size, sound_source: SoundSource, index: usize) -> Box<dyn Window + 'static> {

    let elements = vec![ 
        vector3_field(creation_counter, "position".to_string(), sound_source.position),
        Box::new(StaticStringField::new(format!("range: {}", sound_source.range), StaticStringField::DEFAULT_SIZE)) as _,
    ];

    Box::new(FramedWindow::new(creation_counter, format!("sound source #{}", index), elements, avalible_space)) as _
}

pub fn effect_source_window(creation_counter: &mut CreationCounter, avalible_space: Size, effect_source: EffectSource, index: usize) -> Box<dyn Window + 'static> {

    let elements = vec![ 
        vector3_field(creation_counter, "position".to_string(), effect_source.position),
        {
            let mut elements = Vec::new();

            for (index, particle) in effect_source.particles.into_iter().rev().enumerate() {

                let fields = vec![ 
                    vector3_field(creation_counter, "position".to_string(), particle.position),
                    color_field(creation_counter, "light color".to_string(), particle.light_color), 
                    Box::new(StaticStringField::new(format!("light range: {}", particle.light_range), StaticStringField::DEFAULT_SIZE)) as _,
                ];

                elements.push(Box::new(Expandable::new(creation_counter, index.to_string(), fields)) as _);
            }

            Box::new(Expandable::new(creation_counter, "particles".to_string(), elements)) as _
        },
        Box::new(StaticStringField::new(format!("timer: {}", effect_source.spawn_timer), StaticStringField::DEFAULT_SIZE)) as _,
    ];

    Box::new(FramedWindow::new(creation_counter, format!("effect source #{}", index), elements, avalible_space)) as _
}

pub fn particle_window(creation_counter: &mut CreationCounter, avalible_space: Size, particle: Particle, index: usize, particle_index: usize) -> Box<dyn Window + 'static> {

    let elements = vec![ 
        vector3_field(creation_counter, "position".to_string(), particle.position),
        color_field(creation_counter, "light color".to_string(), particle.light_color), 
        Box::new(StaticStringField::new(format!("light range: {}", particle.light_range), StaticStringField::DEFAULT_SIZE)) as _,
    ];

    Box::new(FramedWindow::new(creation_counter, format!("particle #{} (effect source #{})", particle_index, index), elements, avalible_space)) as _
}
