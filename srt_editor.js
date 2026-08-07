// srt_editor.js
const fs = require('fs');
const readline = require('readline');

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

function parseTime(s) {
    s = s.trim().replace(',', '.');
    const parts = s.split(':');
    if (parts.length !== 3) return 0;
    const h = parseInt(parts[0]);
    const m = parseInt(parts[1]);
    const secParts = parts[2].split('.');
    const sec = parseInt(secParts[0]);
    const ms = secParts.length > 1 ? parseInt(secParts[1]) : 0;
    return h * 3600 + m * 60 + sec + ms / 1000;
}

function formatTime(t) {
    const h = Math.floor(t / 3600);
    const m = Math.floor((t % 3600) / 60);
    const s = Math.floor(t % 60);
    const ms = Math.round((t - Math.floor(t)) * 1000);
    return `${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}:${String(s).padStart(2,'0')},${String(ms).padStart(3,'0')}`;
}

function parseSRT(content) {
    const subs = [];
    const blocks = content.trim().split(/\n\s*\n/);
    for (const block of blocks) {
        const lines = block.split('\n');
        if (lines.length < 3) continue;
        let idx = parseInt(lines[0]);
        const timeLine = lines[1];
        const parts = timeLine.split('-->');
        if (parts.length !== 2) continue;
        const start = parseTime(parts[0]);
        const end = parseTime(parts[1]);
        const text = lines.slice(2).join('\n');
        subs.push({ index: idx, start, end, text });
    }
    subs.forEach((s, i) => s.index = i+1);
    return subs;
}

function formatSRT(subs) {
    return subs.map(s => `${s.index}\n${formatTime(s.start)} --> ${formatTime(s.end)}\n${s.text}`).join('\n\n') + '\n';
}

class Editor {
    constructor(filename) {
        this.filename = filename;
        this.subs = parseSRT(fs.readFileSync(filename, 'utf8'));
        console.log(`Загружено ${this.subs.length} субтитров.`);
    }
    save(fname) {
        fname = fname || this.filename;
        fs.writeFileSync(fname, formatSRT(this.subs), 'utf8');
        console.log('Сохранено.');
    }
    shift(ms) {
        const delta = ms / 1000;
        this.subs.forEach(s => { s.start += delta; s.end += delta; });
        console.log(`Сдвиг на ${ms} мс выполнен.`);
    }
    renumber() {
        this.subs.forEach((s, i) => s.index = i+1);
        console.log('Перенумерация выполнена.');
    }
    validate() {
        const errs = [];
        if (this.subs.length === 0) errs.push('Нет субтитров.');
        let prevEnd = 0;
        this.subs.forEach((s, i) => {
            if (s.start >= s.end) errs.push(`#${i+1}: начало >= конец`);
            if (i > 0 && s.start < prevEnd) errs.push(`#${i+1}: перекрытие с предыдущим`);
            prevEnd = s.end;
        });
        return errs;
    }
    list() {
        this.subs.forEach(s => {
            console.log(`#${s.index}  ${formatTime(s.start)} --> ${formatTime(s.end)}\n${s.text}\n`);
        });
    }
    add(startStr, endStr, text) {
        const start = parseTime(startStr);
        const end = parseTime(endStr);
        this.subs.push({ index: this.subs.length+1, start, end, text });
        this.renumber();
        console.log('Субтитр добавлен.');
    }
    edit(idx, startStr, endStr, text) {
        if (idx < 1 || idx > this.subs.length) { console.log('Неверный номер.'); return; }
        const s = this.subs[idx-1];
        if (startStr && startStr.trim()) s.start = parseTime(startStr);
        if (endStr && endStr.trim()) s.end = parseTime(endStr);
        if (text && text.trim()) s.text = text;
        console.log('Субтитр обновлён.');
    }
    delete(idx) {
        if (idx < 1 || idx > this.subs.length) { console.log('Неверный номер.'); return; }
        this.subs.splice(idx-1, 1);
        this.renumber();
        console.log('Субтитр удалён.');
    }
}

function main() {
    const args = process.argv.slice(2);
    if (args.length === 0) {
        console.log('Использование: node srt_editor.js [--shift мс] [--check] [--output file] файл.srt');
        return;
    }
    let filename = null;
    let shiftMs = 0;
    let checkOnly = false;
    let outputFile = null;
    for (let i=0; i<args.length; i++) {
        if (args[i] === '--shift' && i+1 < args.length) {
            shiftMs = parseInt(args[++i]);
        } else if (args[i] === '--check') {
            checkOnly = true;
        } else if (args[i] === '--output' && i+1 < args.length) {
            outputFile = args[++i];
        } else {
            filename = args[i];
        }
    }
    if (!filename) {
        console.log('Не указан файл.');
        return;
    }
    let editor;
    try {
        editor = new Editor(filename);
    } catch (e) {
        console.log('Ошибка загрузки:', e.message);
        return;
    }
    if (checkOnly) {
        const errs = editor.validate();
        if (errs.length === 0) console.log('Субтитры корректны.');
        else {
            console.log('Ошибки:');
            errs.forEach(e => console.log('  ' + e));
        }
        return;
    }
    if (shiftMs !== 0) {
        editor.shift(shiftMs);
        editor.save(outputFile || filename);
        return;
    }

    console.log('Редактор субтитров. Введите help для списка команд.');
    const prompt = () => {
        rl.question('> ', (cmd) => {
            cmd = cmd.trim();
            if (!cmd) { prompt(); return; }
            const parts = cmd.split(/\s+/);
            const op = parts[0];
            switch (op) {
                case 'quit': rl.close(); return;
                case 'help':
                    console.log('list, show <номер>, add, edit <номер> [время_начала] [время_конца] [текст], delete <номер>, shift <мс>, renumber, validate, save, quit');
                    break;
                case 'list': editor.list(); break;
                case 'show':
                    if (parts.length < 2) { console.log('Укажите номер.'); break; }
                    const idx = parseInt(parts[1]);
                    if (idx<1 || idx>editor.subs.length) { console.log('Неверный номер.'); break; }
                    const s = editor.subs[idx-1];
                    console.log(`#${s.index}  ${formatTime(s.start)} --> ${formatTime(s.end)}\n${s.text}`);
                    break;
                case 'add': {
                    rl.question('Начало (HH:MM:SS,mmm): ', (start) => {
                        rl.question('Конец (HH:MM:SS,mmm): ', (end) => {
                            console.log('Текст (введите END для завершения):');
                            let lines = [];
                            const readText = () => {
                                rl.question('', (line) => {
                                    if (line === 'END') {
                                        editor.add(start, end, lines.join('\n'));
                                        prompt();
                                    } else {
                                        lines.push(line);
                                        readText();
                                    }
                                });
                            };
                            readText();
                        });
                    });
                    return;
                }
                case 'edit':
                    if (parts.length < 2) { console.log('Укажите номер.'); break; }
                    const idx2 = parseInt(parts[1]);
                    rl.question('Новое начало (оставьте пустым): ', (start) => {
                        rl.question('Новый конец (оставьте пустым): ', (end) => {
                            rl.question('Новый текст (оставьте пустым): ', (text) => {
                                editor.edit(idx2, start, end, text);
                                prompt();
                            });
                        });
                    });
                    return;
                case 'delete':
                    if (parts.length < 2) { console.log('Укажите номер.'); break; }
                    editor.delete(parseInt(parts[1]));
                    break;
                case 'shift':
                    if (parts.length < 2) { console.log('Укажите сдвиг в мс.'); break; }
                    editor.shift(parseInt(parts[1]));
                    break;
                case 'renumber': editor.renumber(); break;
                case 'validate':
                    const errs = editor.validate();
                    if (errs.length === 0) console.log('OK');
                    else {
                        console.log('Ошибки:');
                        errs.forEach(e => console.log('  ' + e));
                    }
                    break;
                case 'save':
                    const fname = parts.length > 1 ? parts[1] : null;
                    editor.save(fname);
                    break;
                default: console.log('Неизвестная команда.');
            }
            prompt();
        });
    };
    prompt();
}

main();
