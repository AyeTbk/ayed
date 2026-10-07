# Nextcore analysis

## Brainstorm

### Commands

Soft/Hard command failure
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
whatever necessary for comfortable editing.

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
[TODO elaborate how.]
$[ cmd ; cmd-fails ; final-cmd ]
$[ cmd ; $try[ cmd-fails ] $catch[ cmd-else ] ; final-cmd ]
The above has implications for the config data format,
going from list of strs to list of (str|list of strs)s


### Async commands

TODO?


## Config

Entirely expressed as modules that can be loaded or unloaded cleanly.
Modules may come from disk or from memory.


### Data representation

Old entry data representation is basically Vec<String>.
New entry data representation would become [TODO consider commands needs]
