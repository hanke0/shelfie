export type LanguageOption = {
  code: string;
  name: string;
  nativeName?: string;
};

/** 置顶语言（ISO 639-1） */
export const PINNED_LANGUAGE_CODES = ["zh", "en"] as const;

const ENTRIES: LanguageOption[] = [
  { code: "aa", name: "Afar" },
  { code: "ab", name: "Abkhazian" },
  { code: "ae", name: "Avestan" },
  { code: "af", name: "Afrikaans" },
  { code: "ak", name: "Akan" },
  { code: "am", name: "Amharic", nativeName: "አማርኛ" },
  { code: "an", name: "Aragonese" },
  { code: "ar", name: "Arabic", nativeName: "العربية" },
  { code: "as", name: "Assamese" },
  { code: "av", name: "Avaric" },
  { code: "ay", name: "Aymara" },
  { code: "az", name: "Azerbaijani" },
  { code: "ba", name: "Bashkir" },
  { code: "be", name: "Belarusian" },
  { code: "bg", name: "Bulgarian", nativeName: "български" },
  { code: "bi", name: "Bislama" },
  { code: "bm", name: "Bambara" },
  { code: "bn", name: "Bengali", nativeName: "বাংলা" },
  { code: "bo", name: "Tibetan", nativeName: "བོད་སྐད" },
  { code: "br", name: "Breton" },
  { code: "bs", name: "Bosnian" },
  { code: "ca", name: "Catalan", nativeName: "català" },
  { code: "ce", name: "Chechen" },
  { code: "ch", name: "Chamorro" },
  { code: "co", name: "Corsican" },
  { code: "cr", name: "Cree" },
  { code: "cs", name: "Czech", nativeName: "čeština" },
  { code: "cu", name: "Church Slavic" },
  { code: "cv", name: "Chuvash" },
  { code: "cy", name: "Welsh", nativeName: "Cymraeg" },
  { code: "da", name: "Danish", nativeName: "dansk" },
  { code: "de", name: "German", nativeName: "Deutsch" },
  { code: "dv", name: "Divehi" },
  { code: "dz", name: "Dzongkha" },
  { code: "ee", name: "Ewe" },
  { code: "el", name: "Greek", nativeName: "Ελληνικά" },
  { code: "en", name: "English" },
  { code: "eo", name: "Esperanto" },
  { code: "es", name: "Spanish", nativeName: "español" },
  { code: "et", name: "Estonian", nativeName: "eesti" },
  { code: "eu", name: "Basque", nativeName: "euskara" },
  { code: "fa", name: "Persian", nativeName: "فارسی" },
  { code: "ff", name: "Fulah" },
  { code: "fi", name: "Finnish", nativeName: "suomi" },
  { code: "fj", name: "Fijian" },
  { code: "fo", name: "Faroese" },
  { code: "fr", name: "French", nativeName: "français" },
  { code: "fy", name: "Western Frisian" },
  { code: "ga", name: "Irish", nativeName: "Gaeilge" },
  { code: "gd", name: "Scottish Gaelic" },
  { code: "gl", name: "Galician", nativeName: "galego" },
  { code: "gn", name: "Guarani" },
  { code: "gu", name: "Gujarati", nativeName: "ગુજરાતી" },
  { code: "gv", name: "Manx" },
  { code: "ha", name: "Hausa" },
  { code: "he", name: "Hebrew", nativeName: "עברית" },
  { code: "hi", name: "Hindi", nativeName: "हिन्दी" },
  { code: "ho", name: "Hiri Motu" },
  { code: "hr", name: "Croatian", nativeName: "hrvatski" },
  { code: "ht", name: "Haitian Creole" },
  { code: "hu", name: "Hungarian", nativeName: "magyar" },
  { code: "hy", name: "Armenian", nativeName: "Հայերեն" },
  { code: "hz", name: "Herero" },
  { code: "ia", name: "Interlingua" },
  { code: "id", name: "Indonesian", nativeName: "Bahasa Indonesia" },
  { code: "ie", name: "Interlingue" },
  { code: "ig", name: "Igbo" },
  { code: "ii", name: "Sichuan Yi" },
  { code: "ik", name: "Inupiaq" },
  { code: "io", name: "Ido" },
  { code: "is", name: "Icelandic", nativeName: "íslenska" },
  { code: "it", name: "Italian", nativeName: "italiano" },
  { code: "iu", name: "Inuktitut" },
  { code: "ja", name: "Japanese", nativeName: "日本語" },
  { code: "jv", name: "Javanese" },
  { code: "ka", name: "Georgian", nativeName: "ქართული" },
  { code: "kg", name: "Kongo" },
  { code: "ki", name: "Kikuyu" },
  { code: "kj", name: "Kuanyama" },
  { code: "kk", name: "Kazakh", nativeName: "қазақ" },
  { code: "kl", name: "Kalaallisut" },
  { code: "km", name: "Khmer", nativeName: "ខ្មែរ" },
  { code: "kn", name: "Kannada", nativeName: "ಕನ್ನಡ" },
  { code: "ko", name: "Korean", nativeName: "한국어" },
  { code: "kr", name: "Kanuri" },
  { code: "ks", name: "Kashmiri" },
  { code: "ku", name: "Kurdish" },
  { code: "kv", name: "Komi" },
  { code: "kw", name: "Cornish" },
  { code: "ky", name: "Kyrgyz" },
  { code: "la", name: "Latin", nativeName: "latina" },
  { code: "lb", name: "Luxembourgish" },
  { code: "lg", name: "Ganda" },
  { code: "li", name: "Limburgish" },
  { code: "ln", name: "Lingala" },
  { code: "lo", name: "Lao", nativeName: "ລາວ" },
  { code: "lt", name: "Lithuanian", nativeName: "lietuvių" },
  { code: "lu", name: "Luba-Katanga" },
  { code: "lv", name: "Latvian", nativeName: "latviešu" },
  { code: "mg", name: "Malagasy" },
  { code: "mh", name: "Marshallese" },
  { code: "mi", name: "Māori" },
  { code: "mk", name: "Macedonian", nativeName: "македонски" },
  { code: "ml", name: "Malayalam", nativeName: "മലയാളം" },
  { code: "mn", name: "Mongolian", nativeName: "монгол" },
  { code: "mr", name: "Marathi", nativeName: "मराठी" },
  { code: "ms", name: "Malay", nativeName: "Bahasa Melayu" },
  { code: "mt", name: "Maltese" },
  { code: "my", name: "Burmese", nativeName: "မြန်မာ" },
  { code: "na", name: "Nauru" },
  { code: "nb", name: "Norwegian Bokmål", nativeName: "norsk bokmål" },
  { code: "nd", name: "North Ndebele" },
  { code: "ne", name: "Nepali", nativeName: "नेपाली" },
  { code: "ng", name: "Ndonga" },
  { code: "nl", name: "Dutch", nativeName: "Nederlands" },
  { code: "nn", name: "Norwegian Nynorsk", nativeName: "norsk nynorsk" },
  { code: "no", name: "Norwegian", nativeName: "norsk" },
  { code: "nr", name: "South Ndebele" },
  { code: "nv", name: "Navajo" },
  { code: "ny", name: "Chichewa" },
  { code: "oc", name: "Occitan" },
  { code: "oj", name: "Ojibwa" },
  { code: "om", name: "Oromo" },
  { code: "or", name: "Odia", nativeName: "ଓଡ଼ିଆ" },
  { code: "os", name: "Ossetian" },
  { code: "pa", name: "Punjabi", nativeName: "ਪੰਜਾਬੀ" },
  { code: "pi", name: "Pali" },
  { code: "pl", name: "Polish", nativeName: "polski" },
  { code: "ps", name: "Pashto", nativeName: "پښتو" },
  { code: "pt", name: "Portuguese", nativeName: "português" },
  { code: "qu", name: "Quechua" },
  { code: "rm", name: "Romansh" },
  { code: "rn", name: "Rundi" },
  { code: "ro", name: "Romanian", nativeName: "română" },
  { code: "ru", name: "Russian", nativeName: "русский" },
  { code: "rw", name: "Kinyarwanda" },
  { code: "sa", name: "Sanskrit" },
  { code: "sc", name: "Sardinian" },
  { code: "sd", name: "Sindhi" },
  { code: "se", name: "Northern Sami" },
  { code: "sg", name: "Sango" },
  { code: "si", name: "Sinhala", nativeName: "සිංහල" },
  { code: "sk", name: "Slovak", nativeName: "slovenčina" },
  { code: "sl", name: "Slovenian", nativeName: "slovenščina" },
  { code: "sm", name: "Samoan" },
  { code: "sn", name: "Shona" },
  { code: "so", name: "Somali" },
  { code: "sq", name: "Albanian", nativeName: "shqip" },
  { code: "sr", name: "Serbian", nativeName: "српски" },
  { code: "ss", name: "Swati" },
  { code: "st", name: "Southern Sotho" },
  { code: "su", name: "Sundanese" },
  { code: "sv", name: "Swedish", nativeName: "svenska" },
  { code: "sw", name: "Swahili", nativeName: "Kiswahili" },
  { code: "ta", name: "Tamil", nativeName: "தமிழ்" },
  { code: "te", name: "Telugu", nativeName: "తెలుగు" },
  { code: "tg", name: "Tajik" },
  { code: "th", name: "Thai", nativeName: "ไทย" },
  { code: "ti", name: "Tigrinya" },
  { code: "tk", name: "Turkmen" },
  { code: "tl", name: "Tagalog", nativeName: "Filipino" },
  { code: "tn", name: "Tswana" },
  { code: "to", name: "Tongan" },
  { code: "tr", name: "Turkish", nativeName: "Türkçe" },
  { code: "ts", name: "Tsonga" },
  { code: "tt", name: "Tatar" },
  { code: "tw", name: "Twi" },
  { code: "ty", name: "Tahitian" },
  { code: "ug", name: "Uyghur", nativeName: "ئۇيغۇرچە" },
  { code: "uk", name: "Ukrainian", nativeName: "українська" },
  { code: "ur", name: "Urdu", nativeName: "اردو" },
  { code: "uz", name: "Uzbek" },
  { code: "ve", name: "Venda" },
  { code: "vi", name: "Vietnamese", nativeName: "Tiếng Việt" },
  { code: "vo", name: "Volapük" },
  { code: "wa", name: "Walloon" },
  { code: "wo", name: "Wolof" },
  { code: "xh", name: "Xhosa" },
  { code: "yi", name: "Yiddish", nativeName: "ייִדיש" },
  { code: "yo", name: "Yoruba" },
  { code: "za", name: "Zhuang" },
  { code: "zh", name: "Chinese", nativeName: "中文" },
  { code: "zu", name: "Zulu", nativeName: "isiZulu" },
];

const byCode = new Map(ENTRIES.map((e) => [e.code, e]));
const pinnedSet = new Set<string>(PINNED_LANGUAGE_CODES);

const pinned = PINNED_LANGUAGE_CODES.map((c) => byCode.get(c)).filter(
  (e): e is LanguageOption => e !== undefined,
);

const rest = ENTRIES.filter((e) => !pinnedSet.has(e.code)).sort((a, b) =>
  a.name.localeCompare(b.name, "en"),
);

export const ISO639_1_LANGUAGES: LanguageOption[] = [...pinned, ...rest];

export function getLanguageOption(code: string): LanguageOption | undefined {
  return byCode.get(code.toLowerCase());
}

export function formatLanguageLabel(opt: LanguageOption): string {
  if (opt.nativeName && opt.nativeName !== opt.name) {
    return `${opt.nativeName} (${opt.code})`;
  }
  return `${opt.name} (${opt.code})`;
}

/** 展示已存储的语言（ISO 代码或旧版自由文本） */
export function displayLanguageValue(value: string | undefined | null): string {
  const v = value?.trim();
  if (!v) return "—";
  const opt = getLanguageOption(v);
  return opt ? formatLanguageLabel(opt) : v;
}

export function filterLanguageOptions(query: string): LanguageOption[] {
  const q = query.trim().toLowerCase();
  if (!q) return ISO639_1_LANGUAGES;
  return ISO639_1_LANGUAGES.filter(
    (opt) =>
      opt.code.includes(q) ||
      opt.name.toLowerCase().includes(q) ||
      opt.nativeName?.toLowerCase().includes(q),
  );
}

export function resolveLanguageCode(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return "";
  const lower = trimmed.toLowerCase();
  const exact = getLanguageOption(lower);
  if (exact) return exact.code;
  const byLabel = ISO639_1_LANGUAGES.find(
    (opt) =>
      formatLanguageLabel(opt).toLowerCase() === lower ||
      opt.name.toLowerCase() === lower ||
      opt.nativeName?.toLowerCase() === lower,
  );
  return byLabel?.code ?? null;
}
