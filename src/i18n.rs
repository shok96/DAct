//! Русские строки. Формулировки зависят от ОС: Проводник / Finder,
//! область уведомлений / строка меню / системный лоток, Alt / Option.

pub fn file_manager() -> &'static str {
    #[cfg(windows)]
    {
        "Проводник"
    }
    #[cfg(target_os = "macos")]
    {
        "Finder"
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        "файловый менеджер"
    }
}

pub fn tray_place() -> &'static str {
    #[cfg(windows)]
    {
        "области уведомлений"
    }
    #[cfg(target_os = "macos")]
    {
        "строке меню"
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        "системном лотке"
    }
}

pub fn tray_place_from() -> &'static str {
    #[cfg(windows)]
    {
        "области уведомлений"
    }
    #[cfg(target_os = "macos")]
    {
        "строки меню"
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        "системного лотка"
    }
}

pub fn quit_app() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Завершить DAct"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Выйти из DAct"
    }
}

pub fn mouse_press() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "нажмите кнопку мыши"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "нажмите левую кнопку мыши"
    }
}

pub fn chord(trigger: &str) -> String {
    #[cfg(target_os = "macos")]
    {
        format!("{trigger} + клик")
    }
    #[cfg(not(target_os = "macos"))]
    {
        format!("{trigger} + ЛКМ")
    }
}

pub fn show_in_folder() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Показать в Finder"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Показать в папке"
    }
}

pub fn start_at_login() -> &'static str {
    #[cfg(windows)]
    {
        "Запускать DAct при входе в систему"
    }
    #[cfg(target_os = "macos")]
    {
        "Открывать DAct при входе в учётную запись"
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        "Запускать DAct при входе в сеанс"
    }
}

pub fn plural<'a>(n: usize, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    let n = n % 100;
    let n1 = n % 10;
    if (11..=14).contains(&n) {
        many
    } else if n1 == 1 {
        one
    } else if (2..=4).contains(&n1) {
        few
    } else {
        many
    }
}

pub fn files_n(n: usize) -> String {
    format!("{n} {}", plural(n, "файл", "файла", "файлов"))
}

pub fn images_n(n: usize) -> String {
    format!("{n} {}", plural(n, "изображение", "изображения", "изображений"))
}

pub fn pages_n(n: usize) -> String {
    format!("{n} {}", plural(n, "страница", "страницы", "страниц"))
}

pub fn pdfs_n(n: usize) -> String {
    format!("{n} {}", plural(n, "PDF", "PDF", "PDF"))
}

pub fn ring_convert() -> &'static str {
    "Формат"
}
pub fn ring_tools() -> &'static str {
    "Инструменты"
}
pub fn ring_audio() -> &'static str {
    "Аудио"
}
pub fn ring_video() -> &'static str {
    "Видео"
}
pub fn ring_extract() -> &'static str {
    "Звук"
}
pub fn ring_one_type() -> &'static str {
    "Один тип"
}
pub fn ring_unsupported() -> &'static str {
    "Нельзя"
}

pub fn tool_compress() -> &'static str {
    "Сжать"
}
pub fn tool_resize() -> &'static str {
    "Размер"
}
pub fn tool_rotate() -> &'static str {
    "Поворот"
}
pub fn tool_metadata() -> &'static str {
    "Метки"
}
pub fn tool_split() -> &'static str {
    "Разбить"
}
pub fn tool_merge() -> &'static str {
    "Склеить"
}
pub fn tool_extract() -> &'static str {
    "Кадры"
}

pub fn hint_ffmpeg() -> &'static str {
    "Через FFmpeg"
}
pub fn hint_audio_only() -> &'static str {
    "Только звук"
}
pub fn hint_extract_text() -> &'static str {
    "Текст из PDF"
}
pub fn hint_plain_text() -> &'static str {
    "Простой текст"
}
pub fn hint_compress_image() -> &'static str {
    "Меньше вес файла"
}
pub fn hint_resize() -> &'static str {
    "Другой размер"
}
pub fn hint_rotate() -> &'static str {
    "На 90°"
}
pub fn hint_metadata_image() -> &'static str {
    "Убрать камеру и GPS"
}
pub fn hint_compress_pdf() -> &'static str {
    "Перезаписать, убрать лишнее"
}
pub fn hint_split() -> &'static str {
    "По файлу на страницу"
}
pub fn hint_rotate_pdf() -> &'static str {
    "Страницы на 90°"
}
pub fn hint_merge() -> &'static str {
    "В один PDF"
}
pub fn hint_extract() -> &'static str {
    "Достать картинки"
}
pub fn hint_metadata_pdf() -> &'static str {
    "Копия без меток"
}

pub fn images_to_pdf_hint(n: usize) -> String {
    format!("{} → 1 PDF", images_n(n))
}

pub fn format_hint(target: &str) -> &'static str {
    match target {
        "JPG" => "Малый вес, везде открывается",
        "PNG" => "Без потерь · прозрачность",
        "WEBP" => "Для веба",
        "TIFF" => "Печать и архив",
        "BMP" => "Без сжатия",
        "PDF" => "Документ",
        "ICO" => ico_hint(),
        "GIF" => "Старый веб",
        "DOCX" => "Редактируемый Word",
        "TXT" => "Простой текст",
        "HEVC" => "H.265 · меньше вес",
        "ProRes" => "Монтаж на Mac",
        "MPEG" => "Классический MPEG",
        "TS" => "Поток MPEG-TS",
        _ => "",
    }
}

fn ico_hint() -> &'static str {
    #[cfg(windows)]
    {
        "Значок Windows"
    }
    #[cfg(target_os = "macos")]
    {
        "Значок .ico"
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        "Значок"
    }
}

pub fn category_label(cat: crate::category::Category) -> &'static str {
    use crate::category::Category::*;
    match cat {
        Image => "Изображение",
        Pdf => "PDF",
        Docx => "Word",
        Text => "Текст",
        Audio => "Аудио",
        Video => "Видео",
    }
}

pub fn drop_file() -> &'static str {
    "Перетащите файл"
}
pub fn one_type_only() -> &'static str {
    "Только один тип"
}

pub fn suffix_compressed() -> &'static str {
    "сжатый"
}
pub fn suffix_rotated() -> &'static str {
    "повёрнутый"
}
pub fn suffix_clean() -> &'static str {
    "чистый"
}
pub fn suffix_combined() -> &'static str {
    "сборка"
}
pub fn suffix_merged() -> &'static str {
    "склейка"
}
pub fn suffix_hevc() -> &'static str {
    "HEVC"
}
pub fn suffix_prores() -> &'static str {
    "ProRes"
}
pub fn suffix_page(n: usize) -> String {
    format!("стр. {n}")
}
pub fn suffix_frame(n: u32) -> String {
    format!("кадр {n}")
}

pub fn settings_title() -> &'static str {
    "Настройки"
}
pub fn settings_hotkey() -> &'static str {
    "Горячая клавиша"
}
pub fn settings_hotkey_help(trigger: &str) -> String {
    #[cfg(target_os = "macos")]
    {
        format!(
            "Удерживайте {trigger} и {}. Control не вызывает меню Finder: клик забирает DAct.",
            mouse_press()
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        format!(
            "Удерживайте {trigger} и {}. Можно сменить в любой момент.",
            mouse_press()
        )
    }
}
pub fn settings_save() -> &'static str {
    "Куда писать результат"
}
pub fn settings_next_to() -> &'static str {
    "Рядом с оригиналом"
}
pub fn settings_subfolder() -> &'static str {
    "В подпапку DAct"
}
pub fn settings_jpeg() -> &'static str {
    "Качество JPEG"
}
pub fn settings_audio() -> &'static str {
    "Качество звука"
}
pub fn quality_low() -> &'static str {
    "Низкое"
}
pub fn quality_medium() -> &'static str {
    "Среднее"
}
pub fn quality_high() -> &'static str {
    "Высокое"
}
pub fn settings_ffmpeg() -> &'static str {
    "Путь к FFmpeg (необязательно)"
}
pub fn settings_ffmpeg_help() -> &'static str {
    "Только локальный файл. «Скачать FFmpeg» кладёт сборку в папку пользователя. При запуске ничего не качается."
}
pub fn browse() -> &'static str {
    "Обзор"
}
pub fn ffmpeg_exe_title() -> &'static str {
    "Исполняемый файл FFmpeg"
}

pub fn welcome_title() -> &'static str {
    "Как пользоваться DAct"
}
pub fn welcome_lead() -> &'static str {
    "Медное кольцо вместо стеклянного диска."
}
pub fn welcome_step1() -> &'static str {
    #[cfg(windows)]
    {
        "1. Выберите файл в Проводнике или начните перетаскивать."
    }
    #[cfg(target_os = "macos")]
    {
        "1. Выберите файл в Finder или начните перетаскивать."
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        "1. Перетащите файл в окно или нажмите «Открыть файлы»."
    }
}
pub fn welcome_step2(trigger: &str) -> String {
    format!(
        "2. Удерживайте {trigger} и {}. Кольцо откроется под курсором.",
        mouse_press()
    )
}
pub fn welcome_step3() -> &'static str {
    "3. Не отпуская кнопку, наведите на сектор и отпустите. Исходный файл не меняется."
}
pub fn welcome_step4(trigger: &str) -> String {
    format!("4. Ещё раз нажмите {trigger} — переключение Формат ↔ Инструменты.")
}
pub fn welcome_step5() -> String {
    #[cfg(target_os = "macos")]
    {
        format!(
            "5. Закройте или сверните окно — DAct останется в {}. Завершить программу можно только оттуда.",
            tray_place()
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        format!(
            "5. Закройте или сверните окно — DAct останется в {}. Выйти можно только из значка.",
            tray_place()
        )
    }
}
pub fn welcome_privacy() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Ход работы и результат — в уведомлениях. DAct не снимает экран. Для Option и чтобы Control не открывал меню Finder, разрешите доступ в Настройки → Конфиденциальность и безопасность → Универсальный доступ."
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Ход работы и результат приходят уведомлениями. DAct не снимает экран и никуда не стучится. FFmpeg качается только по кнопке «Скачать FFmpeg»."
    }
}
pub fn got_it() -> &'static str {
    "Понятно"
}

pub fn how_to_use() -> &'static str {
    "Как пользоваться"
}
pub fn get_ffmpeg() -> &'static str {
    "Скачать FFmpeg"
}
pub fn open_files() -> &'static str {
    "Открыть файлы"
}
pub fn clear() -> &'static str {
    "Очистить"
}
pub fn drop_files_here() -> &'static str {
    "Перетащите файлы сюда"
}
pub fn queue() -> &'static str {
    "Очередь"
}
pub fn drop_kinds() -> &'static str {
    "Картинки, PDF, текст, аудио, видео."
}
pub fn no_file() -> &'static str {
    "Файл не выбран"
}
pub fn no_file_help() -> String {
    #[cfg(windows)]
    {
        format!(
            "Откройте или перетащите файл. Результаты — в уведомлениях. Кольцо открывается горячей клавишей из {}.",
            tray_place_from()
        )
    }
    #[cfg(target_os = "macos")]
    {
        format!(
            "Откройте или перетащите файл. Результаты — в уведомлениях. Кольцо открывается горячей клавишей из {}.",
            tray_place_from()
        )
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        format!(
            "Перетащите файл в окно или откройте его. Результаты — в уведомлениях. Кольцо открывается горячей клавишей из {}.",
            tray_place_from()
        )
    }
}
pub fn select_one_type() -> &'static str {
    "Выберите один тип файлов"
}
pub fn select_one_type_help() -> &'static str {
    "Пакет работает, когда все файлы одного рода: только картинки, только видео или только звук."
}
pub fn same_type_n(n: usize) -> String {
    format!("{} · один тип", files_n(n))
}
pub fn need_ffmpeg() -> &'static str {
    "Для аудио и видео нужен FFmpeg в папке пользователя. Нажмите «Скачать FFmpeg» или укажите файл в Настройках."
}
pub fn heic_ffmpeg() -> &'static str {
    "HEIC и AVIF читает встроенный декодер; если не выйдет — локальный FFmpeg."
}
pub fn home_tagline() -> String {
    format!(
        "Сохраняем рядом с оригиналом. Закройте окно — DAct останется в {}.",
        tray_place()
    )
}
pub fn ffmpeg_ready_chip() -> &'static str {
    "есть"
}
pub fn ffmpeg_missing_chip() -> &'static str {
    "нет"
}
pub fn empty_formats_images() -> &'static str {
    "PNG JPG WEBP TIFF BMP GIF ICO"
}
pub fn empty_formats_heic() -> &'static str {
    "HEIC AVIF  ·  нужен FFmpeg"
}
pub fn empty_formats_media() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "MP3 WAV FLAC MP4 MOV HEVC  ·  нужен FFmpeg"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "MP3 WAV FLAC MP4 MKV  ·  нужен FFmpeg"
    }
}
pub fn empty_formats_docs() -> &'static str {
    "PDF TXT MD DOCX"
}

pub fn compress_title() -> &'static str {
    "Сжать"
}
pub fn compress_help() -> &'static str {
    "Качество новой копии. Оригинал не трогаем."
}
pub fn compress_slider() -> &'static str {
    "качество"
}
pub fn resize_title() -> &'static str {
    "Размер"
}
pub fn resize_pct() -> &'static str {
    "Проценты"
}
pub fn resize_long() -> &'static str {
    "Длинная сторона"
}
pub fn resize_w() -> &'static str {
    "Ширина"
}
pub fn resize_h() -> &'static str {
    "Высота"
}
pub fn resize_value() -> &'static str {
    "значение"
}

pub fn app_version(version: &str) -> String {
    format!("Версия {version}")
}

pub fn tray_show() -> &'static str {
    "Показать DAct"
}
pub fn tray_enabled() -> &'static str {
    "Включено"
}
pub fn tray_open() -> &'static str {
    "Открыть файл…"
}
pub fn tray_settings() -> &'static str {
    "Настройки…"
}
pub fn tray_tip(trigger: &str) -> String {
    #[cfg(windows)]
    {
        format!("DAct — {trigger} + ЛКМ по файлу в Проводнике")
    }
    #[cfg(target_os = "macos")]
    {
        format!("DAct — {trigger} + клик по файлу в Finder")
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        format!("DAct — {trigger} + ЛКМ · перетащите файл на кольцо")
    }
}

pub fn ready_title() -> &'static str {
    "DAct готов"
}
pub fn ready_body(trigger: &str) -> String {
    format!(
        "Перетащите файл сюда или удерживайте {trigger} и {}. Закройте окно — DAct останется в {}.",
        mouse_press(),
        tray_place()
    )
}
pub fn ffmpeg_ready_title() -> &'static str {
    "FFmpeg готов"
}
pub fn ffmpeg_ready_body() -> &'static str {
    "Можно конвертировать видео и звук."
}
pub fn getting_ffmpeg() -> &'static str {
    "Скачиваем FFmpeg…"
}
pub fn getting_ffmpeg_detail() -> &'static str {
    "Статическая сборка в папку этого пользователя."
}
pub fn hidden_title() -> &'static str {
    "DAct в фоне"
}
pub fn hidden_body() -> String {
    #[cfg(target_os = "macos")]
    {
        format!(
            "Конвертация продолжается. Окно — из значка в {}. Завершить программу можно только оттуда.",
            tray_place()
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        format!(
            "Конвертация продолжается. Окно — из значка в {}. Выйти можно только оттуда.",
            tray_place()
        )
    }
}
pub fn already_running() -> &'static str {
    "DAct уже запущен"
}
pub fn already_running_body(trigger: &str) -> String {
    format!(
        "Он в {}. Удерживайте {trigger} и {} или откройте окно из значка.",
        tray_place(),
        mouse_press()
    )
}
pub fn couldnt_finish() -> &'static str {
    "Не получилось"
}
pub fn couldnt_get_ffmpeg() -> &'static str {
    "Не удалось получить FFmpeg"
}
pub fn open() -> &'static str {
    "Открыть"
}
pub fn undo() -> &'static str {
    "Отменить"
}

pub fn working_label(action: &str) -> String {
    let (kind, name) = action.split_once(':').unwrap_or(("", action));
    match (kind, name) {
        ("convert", n) | ("media", n) => format!("В {n}…"),
        ("extract", n) => format!("Звук {n}…"),
        ("tool", "Compress") => "Сжимаем…".into(),
        ("tool", "Resize") => "Меняем размер…".into(),
        ("tool", "Rotate") => "Поворачиваем…".into(),
        ("tool", "Split") => "Разбиваем…".into(),
        ("tool", "Merge") => "Склеиваем…".into(),
        ("tool", "Extract") => "Достаём кадры…".into(),
        ("tool", "Metadata") => "Снимаем метки…".into(),
        _ => "Работаем…".into(),
    }
}

pub fn quality_note(q: u8) -> String {
    format!("качество {q}")
}

pub fn progress_many(n: usize, total: usize, name: &str, pct: u32) -> String {
    format!("{n} / {total}  ·  {name}  ·  {pct}%")
}
pub fn progress_one(name: &str, pct: u32) -> String {
    format!("{name}  ·  {pct}%")
}
pub fn fallback_file() -> &'static str {
    "файл"
}

pub fn nothing_written() -> &'static str {
    "Ничего не записано"
}
pub fn converted_to(name: &str) -> String {
    format!("Готово: {name}")
}
pub fn files_to_one(n: usize, name: &str) -> String {
    format!("{} → 1 {name}", files_n(n))
}
pub fn pages_to_files(pages: usize, name: &str) -> String {
    format!("{} → {} {name}", pages_n(pages), files_n(pages))
}
pub fn files_to_fmt(n: usize, name: &str) -> String {
    format!("{} → {name}", files_n(n))
}
pub fn ext_to_fmt(ext: &str, name: &str) -> String {
    format!("{ext} → {name}")
}
pub fn compressed_title() -> &'static str {
    "Сжато"
}
pub fn compressed_n(n: usize) -> String {
    format!("Сжато: {}", files_n(n))
}
pub fn resized_title() -> &'static str {
    "Размер изменён"
}
pub fn rotated_title() -> &'static str {
    "Повёрнуто"
}
pub fn resized_n(n: usize) -> String {
    format!("Изменён размер: {}", images_n(n))
}
pub fn rotated_n(n: usize) -> String {
    format!("Повёрнуто: {}", images_n(n))
}
pub fn metadata_removed() -> &'static str {
    "Метки сняты"
}
pub fn split_into(n: usize) -> String {
    format!("Разбито на {}", pdfs_n(n))
}
pub fn merged_pdfs() -> &'static str {
    "PDF склеены"
}
pub fn extracted_images(n: usize) -> String {
    format!("Достали {}", images_n(n))
}
pub fn private_copy() -> &'static str {
    "копия без меток"
}
pub fn pct_smaller(pct: f64) -> String {
    format!("на {pct:.0}% легче")
}
pub fn already_small() -> &'static str {
    "и так хорошо сжат"
}
pub fn same_size() -> &'static str {
    "тот же размер"
}

pub fn err_no_files() -> &'static str {
    "Нет файлов."
}
pub fn err_one_type() -> &'static str {
    "Выберите файлы одного типа — картинки, видео, звук или PDF."
}
pub fn err_unknown_action() -> &'static str {
    "Неизвестное действие."
}
pub fn err_cant_convert(target: &str) -> String {
    format!("Этот файл нельзя сохранить как {target}.")
}
pub fn err_tool_missing(name: &str) -> String {
    format!("Действие {name} недоступно.")
}
pub fn err_unknown_audio(target: &str) -> String {
    format!("Неизвестный формат звука {target}")
}
pub fn err_unknown_video(target: &str) -> String {
    format!("Неизвестный формат видео {target}")
}
pub fn err_unknown_image(target: &str) -> String {
    format!("Неизвестный формат картинки {target}")
}
pub fn err_no_images_combine() -> &'static str {
    "Нечего собирать в PDF."
}
pub fn err_no_pages() -> &'static str {
    "Нечего записать: страниц нет."
}
pub fn err_pdf_read(e: impl std::fmt::Display) -> String {
    format!("Не удалось прочесть PDF: {e}")
}
pub fn err_merge_two() -> &'static str {
    "Чтобы склеить, перетащите хотя бы два PDF."
}
pub fn err_pdf_empty() -> &'static str {
    "В этом PDF нет страниц."
}
pub fn err_no_embedded() -> &'static str {
    "В этом PDF нет встроенных картинок."
}
pub fn err_pdf_page() -> &'static str {
    "Не удалось прочесть страницу PDF."
}
pub fn err_docx_bad() -> &'static str {
    "Этот Word-файл повреждён или это не .docx."
}
pub fn err_disk_full() -> &'static str {
    "Не хватает места на диске."
}
pub fn err_ffmpeg_missing_bin() -> &'static str {
    "FFmpeg скачан, но в архиве нет программы ffmpeg."
}
pub fn err_ffmpeg_no_mac(arch: &str) -> String {
    format!(
        "Для этого Mac ({arch}) нет готовой сборки FFmpeg. Поставьте ffmpeg сами и укажите путь в Настройках."
    )
}
pub fn err_ffmpeg_no_linux(arch: &str) -> String {
    format!(
        "Для этого Linux ({arch}) нет готовой сборки FFmpeg. Поставьте ffmpeg сами и укажите путь в Настройках."
    )
}
pub fn err_ffmpeg_no_os() -> &'static str {
    "Для этой системы FFmpeg не скачивается. Поставьте ffmpeg сами и укажите путь в Настройках."
}
pub fn err_ffmpeg_download(e: impl std::fmt::Display) -> String {
    format!("Не удалось скачать FFmpeg: {e}")
}
pub fn err_ffmpeg_archive(e: impl std::fmt::Display) -> String {
    format!("Не удалось открыть архив FFmpeg: {e}")
}
pub fn err_ffmpeg_archive_read(e: impl std::fmt::Display) -> String {
    format!("Не удалось прочесть архив FFmpeg: {e}")
}
pub fn err_ffmpeg_no_bin() -> &'static str {
    "В архиве FFmpeg нет программы ffmpeg."
}
pub fn err_ffmpeg_linux_only() -> &'static str {
    "Этот архив FFmpeg распаковывается только в Linux."
}
pub fn err_file_gone() -> &'static str {
    "Файл перенесли или удалили."
}
pub fn err_missing(path: impl std::fmt::Display) -> String {
    format!("Файл перенесли или удалили: {path}")
}
pub fn err_permission() -> &'static str {
    "Нет прав записать рядом с этим файлом. Выберите свою папку или в Настройках включите сохранение в подпапку DAct."
}
pub fn err_unsupported() -> &'static str {
    "DAct не читает этот формат."
}
pub fn err_no_ffmpeg() -> &'static str {
    "FFmpeg не найден. Нажмите «Скачать FFmpeg» или укажите путь в Настройках."
}
pub fn err_ffmpeg_failed() -> &'static str {
    "FFmpeg не смог перекодировать файл в выбранный формат."
}
pub fn err_no_audio() -> &'static str {
    "В этом файле нет звуковой дорожки."
}
pub fn err_pdf_locked() -> &'static str {
    "PDF защищён паролем. Сначала снимите защиту."
}

pub fn meta_size() -> &'static str {
    "Размер"
}
pub fn meta_none() -> &'static str {
    "меток нет"
}
pub fn meta_label() -> &'static str {
    "Метки"
}

pub fn toast_overlay() -> &'static str {
    "Уведомления DAct"
}

pub fn start_failed(e: impl std::fmt::Display) -> String {
    format!("DAct не запустился: {e}")
}

pub fn cli_help(app: &str, version: &str, trigger: &str) -> String {
    let quit = if cfg!(target_os = "macos") {
        "Завершить"
    } else {
        "Выйти"
    };
    format!(
        "{app} {version}
Конвертация файлов кольцевым меню.

Запуск:
  dact                     Приложение в фоне
  dact <файлы...>          Открыть эти файлы
  dact convert <файл> --to JPG
  dact --selftest
  dact --quiet             Только фон, без окна

Выберите файл, удерживайте {trigger}, {press}, отпустите на секторе.
Закройте окно — DAct останется в {tray}. {quit} только из значка.
Исходный файл не меняется.
",
        press = mouse_press(),
        tray = tray_place(),
    )
}

pub fn cli_usage() -> &'static str {
    "использование: dact convert <файл> --to JPG"
}
pub fn cli_no_files() -> &'static str {
    "нет входных файлов"
}
pub fn cli_unsupported() -> &'static str {
    "формат не поддерживается"
}
