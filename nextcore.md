# Nextcore analysis

## Brainstorm

### Commands

Hooking before/after execution
Invocation substitutions

### Uncategorized

Registers
Multieditor
Workspaces
Logical content
Buffer content properties
Panels
UI/UX
UI layout
Rendering Api


## Keep in mind throughout design and development

Panic safety / reduce potential panics / design to avoid panics.


## Overarching architecture

- Core
	- Plugins:
        * Registers all the stuff it offers/needs.
        * Can be unloaded, reloaded.
        - Commands
        - Hooks
        - Config modules
    - State
        * All command-accessible state, runtime borrowchecking for flexibility.
        * Dynamic, dynamic registration, think typeid hashmap.
        - AppliedConfigs
        - Buffers
        - Plugin registered data


## Plugins

The majority of features should be implemented as plugins. 
Plugins can have their own data that gets stored in the core.
Plugins can have their own config modules.
Plugins can be cleanly loaded and unloaded dynamically.
On load (most likely at startup), plugins register one by one their data,
config, commands, ...
On unload (most likely initated by user), core cleans things up.

TODO Plugins interdependency


## DataStorage

All command-accessible state, runtime borrowchecking for flexibility.
Dynamic, dynamic registration, think typeid hashmap.

TODO Should buffer be big and contain a lot, or should stuff be spread out
	 in storage? Like, markers. Should they be in the buffer struct?

## Buffers

Buffers will be addressed uniquely with monotonically increasing ids.

Plz rename type Selections to SelectionSet.


### Markers

They will track "markers", positions within the buffer's content with an
identity of their own, which the buffer updates as its content is modified.

They will also track selections, so in order that have access to selections,
one must query the buffer.

Ranges will represent [start, end) static position pairs.
Selections are like ranges but with markers instead of positions, augmented with
whatever necessary for comfortable editing, like `desired_column`.

Positioning of the following elements will be handled by markers:
syntax highlighting, inline virtual content, ...

Care must be taken not to forget to clean up markers and selections that have
become unused. Either core handles this flawlessly somehow (and what about
plugins?), or some automatic scheme is used (ref counting).


### Virtual content

Virtual lines. Insert themselves between lines. Positioned by a marker with 0-column.
Virtual inline spans. Insert themselves within a line. Positioned by a marker.
Motions/cursors/selections ignore them. Line number display needs to adapt.


### Different buffer types?

Could be neat. Text, Bytes, Spreadsheet, ...
Allow opening the same file multiple times as different buffer types.
Syncing across these buffers of a same file of different buffer type would be
really neat.

How to use polymorphically? Do each type get their own commands (no!), or should
commands generally work with as many types as reasonable?(yes).


## Commands

Commands can fail and emit an error. A native commands should be atomic, so
either the command worked and its effects have happened, or it failed and only
an error message is emitted.

Scripted commands should be able to handle/ignore sub-command failures.
Example:
implicitly define anon command in config:
<s-o> ${
  cmd; cmd2;
  $try{ move u } $catch{ move d };
  $if state regex {
    cmd3; cmd4
  }
}

 becomes

defcmd __cmd_annon23947823 {
  cmd; cmd2;
  $try{ move u } $catch{ move d }; # catch block is optional
  $if state regex { # if is an in-script selector basically
    cmd3; cmd4
  }
}
<s-o> $[ __cmd_annon23947823 ]

Ive meant to have some kind of defcmd for a while, and this allows keeping
configs as lists of strings while allowing more complex scripting constructs.
This implies a more advanced script executor to handle constructs.
Command queueing still 'nests' command execution though.
defcmd scripts should be affected by selectors, that is: a defcmd in an
inactive selector is not accessible/executable.

TODO $(substitutions) in commands


### More advanced script executor

Needs:
- Can run command sequences.
- Can run nested command sequences.
- Can handle $if state regex { etc }
- Can handle $try{} $catch{}
- Can handle executing hooked commands before, after and nested-after
- Updated live as the config changes, of course.


### Async commands

There shouldn't be async commands, and anything that would greatly benefit
from them should just be purpose built in native code. Plugins should allow
this with hooks.


## Config

Entirely expressed as modules that can be loaded or unloaded cleanly.
Modules may come from disk or from memory.

Contexts: focused-view, view, etc. How to represent?
ex maybe: config.for_view(ViewId) -> &AppliedConfig
Are there other contexts than views? maybe global? workspace?
- Anything that could be seen simultaneously while needing separate
  config, like two editors open side by side with files of different
  languages. Would require competing syntax mappings => different contexts.
- Can multiple different things need to respond to input differently and
  simultaneously? (No? focus kinda makes it not a possibility?)
- Background processing of sorts? (like what??)


### Data representation

Old entry data representation is basically Vec<String>.
New entry data representation would become [TODO consider commands needs]
