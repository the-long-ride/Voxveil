# Localization Specification

Bundled locales:

- English (`en`)
- Vietnamese (`vi`)
- Chinese (`zh`)
- Korean (`ko`)
- Japanese (`ja`)
- Spanish (`es`)
- French (`fr`)

All locale JSON files must have exact key parity with English. `npm run quality:i18n` is a hard gate. Translation resources are bundled at build time and no translation service is contacted at runtime.

UI layout must tolerate longer French, Spanish, and Vietnamese labels without fixed-width clipping. Navigation may adapt responsively but may not silently truncate critical controls.

Selected language is stored locally under the Voxveil namespace. If no selection exists, the UI chooses a supported base language from the device locale and falls back to English.
