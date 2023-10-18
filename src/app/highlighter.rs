use clru::CLruCache;
use egui::RichText;

enum HighlightMode {
    Rythm,
    No,
}

struct Highlighter {
    cache_highlight: CLruCache<String, RichText>,
    cache_words: CLruCache<String, RichText>,
    mode: HighlightMode,
}
