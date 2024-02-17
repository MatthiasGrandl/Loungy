# Structure

## workspace.rs:

Contains the base layout

## query.rs

Contains the text input used for both the main search bar and for the action popup

## state.rs

Contains the logic of the state stack

## swift.rs

The rust counterpart for the `swift-lib` functions. Used mostly for MacOS Accessibility API and other stuff I don't know how to do with Rust

## The State Stack

I experimented quite a bit until I landed on this structure. The central component behind all of Loungy is the query. It powers most views (lists and even forms). So actions are bound through the query keydown events. So the State Items look like this

- Query: the text input that powers interactions,
- View: the view that updates based on the interaction,
- Actions: dynamic actions based on the state of the view,
- Toast: a view to send success/error messages on actions,
- Loading: the loading bar when an async view is loaded,
- Workspace: boolean to indicate whether we want this view inserted at the base of the app

If workspace is set to false, it allows us to render a view anywhere we want while still having consistent behavior with the state stack, essentially allowing us to have indefinitely deep nested views! One example and the reason I came up with this:

I wanted to have a matrix chat client in Loungy. The first view is the list of chat rooms which we can fuzzy search. The preview pane renders the currently selected chat room right next to the room list. When we hit enter, it moves the chat room to the state stack, allowing us to interact with the chat room! When we hit back, it removes the chat room from the stack and the focus is back on the chat room list.

You see the flexibility this approach gives us.
