# Editing an existing language

There is an easy way of editing the localization built in to the client.
When running Korangar with debug features enabled, use the `Client State Inspector` and navigate to `localization`.
It contains all the data loaded by the language.
The components in this inspector are mutable, meaning you can change the text used for localization in place and see the results in real time.
To save your changes, simply expand the `controls` at the top and press `Save`.

Alternatively you can edit the language files directly in this directory.

# Adding a new language

### Step 1: Create the language file

Duplicate one of the language files in this directory (likely `en-US.ron`) as e.g. `ja-JP.ron`.
This will ensure you have a valid language file to start with.

### Step 2: Modify the language enum

Add the language to the `Language` enum (in `src/state/localization/mod.rs`) and add the display name and locale code to the `impl` blocks.

The display name should be the name of the language in the language itself.
For example, Japanese should display `日本語` instead of `Japanese`.

The locale code should be a standardized short code for that locale.

### Step 3: Make the language selectable

Add the newly created enum variant to `InterfaceSettingsCapabilities` in `src/settings/interface.rs`.
Search for `English` in the file to find the right location.
Adding this entry makes the new language selectable from the interface settings window.

### Step 4: Fill in the language file

Edit the newly created language. See [Editing an existing language](#editing-an-existing-language) for instructions.

# Adding new fields to the localization

When adding a new window or component you might need to add a new field to the `Localization` struct (in `src/state/localization/mod.rs`).
Any language files in this directory also need this new field before they can be loaded by the client, so you need to add it manually.
For any language that you are _not_ capable of translating, please use in the magic value `???` for the new field instead of using google translate or other tools.
