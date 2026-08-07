// srt_editor.cpp
#include <iostream>
#include <fstream>
#include <string>
#include <vector>
#include <regex>
#include <iomanip>
#include <chrono>
#include <sstream>
#include <cctype>

using namespace std;

struct Subtitle {
    int index;
    chrono::milliseconds start;
    chrono::milliseconds end;
    string text;
};

chrono::milliseconds parseTime(const string& s) {
    // HH:MM:SS,mmm или HH:MM:SS.mmm
    string str = s;
    replace(str.begin(), str.end(), ',', '.');
    int h=0, m=0, sec=0, ms=0;
    char delim;
    stringstream ss(str);
    ss >> h >> delim >> m >> delim >> sec >> delim >> ms;
    if (delim == '.') {
        // ms уже прочитан
    }
    return chrono::milliseconds(h*3600000 + m*60000 + sec*1000 + ms);
}

string formatTime(const chrono::milliseconds& d) {
    auto total_ms = d.count();
    int h = total_ms / 3600000;
    int m = (total_ms % 3600000) / 60000;
    int sec = (total_ms % 60000) / 1000;
    int ms = total_ms % 1000;
    ostringstream oss;
    oss << setfill('0') << setw(2) << h << ":"
        << setw(2) << m << ":"
        << setw(2) << sec << ","
        << setw(3) << ms;
    return oss.str();
}

vector<Subtitle> parseSRT(const string& content) {
    vector<Subtitle> subs;
    regex block_regex(R"((\d+)\s*\n([\d:,]+)\s*-->\s*([\d:,]+)\s*\n([\s\S]*?)(?=\n\n|\Z))",
                      regex::multiline);
    smatch match;
    string::const_iterator start = content.cbegin();
    while (regex_search(start, content.cend(), match, block_regex)) {
        Subtitle sub;
        sub.index = stoi(match[1]);
        sub.start = parseTime(match[2]);
        sub.end = parseTime(match[3]);
        sub.text = match[4];
        subs.push_back(sub);
        start = match.suffix().first;
    }
    // перенумеруем
    for (size_t i=0; i<subs.size(); ++i) subs[i].index = i+1;
    return subs;
}

string formatSRT(const vector<Subtitle>& subs) {
    ostringstream oss;
    for (const auto& sub : subs) {
        oss << sub.index << "\n";
        oss << formatTime(sub.start) << " --> " << formatTime(sub.end) << "\n";
        oss << sub.text << "\n\n";
    }
    return oss.str();
}

class Editor {
public:
    string filename;
    vector<Subtitle> subs;

    bool load(const string& fname) {
        ifstream file(fname);
        if (!file.is_open()) return false;
        string content((istreambuf_iterator<char>(file)), istreambuf_iterator<char>());
        subs = parseSRT(content);
        filename = fname;
        cout << "Загружено " << subs.size() << " субтитров.\n";
        return true;
    }
    bool save(const string& fname = "") {
        string out = (fname.empty() ? filename : fname);
        ofstream file(out);
        if (!file.is_open()) return false;
        file << formatSRT(subs);
        return true;
    }
    void shift(int ms) {
        auto delta = chrono::milliseconds(ms);
        for (auto& sub : subs) {
            sub.start += delta;
            sub.end += delta;
        }
        cout << "Сдвиг на " << ms << " мс выполнен.\n";
    }
    void renumber() {
        for (size_t i=0; i<subs.size(); ++i) subs[i].index = i+1;
        cout << "Перенумерация выполнена.\n";
    }
    vector<string> validate() {
        vector<string> errs;
        if (subs.empty()) errs.push_back("Нет субтитров.");
        chrono::milliseconds prev_end(0);
        for (size_t i=0; i<subs.size(); ++i) {
            if (subs[i].start >= subs[i].end) {
                errs.push_back("#" + to_string(i+1) + ": начало >= конец");
            }
            if (i>0 && subs[i].start < prev_end) {
                errs.push_back("#" + to_string(i+1) + ": перекрытие с предыдущим");
            }
            prev_end = subs[i].end;
        }
        return errs;
    }
    void list() {
        for (const auto& sub : subs) {
            cout << "#" << sub.index << "  " << formatTime(sub.start) << " --> " << formatTime(sub.end) << "\n"
                 << sub.text << "\n\n";
        }
    }
    void add(const string& start_str, const string& end_str, const string& text) {
        auto start = parseTime(start_str);
        auto end = parseTime(end_str);
        subs.push_back({(int)subs.size()+1, start, end, text});
        renumber();
        cout << "Субтитр добавлен.\n";
    }
    void edit(int idx, const string& start_str, const string& end_str, const string& text) {
        if (idx<1 || idx>(int)subs.size()) { cout << "Неверный номер.\n"; return; }
        auto& sub = subs[idx-1];
        if (!start_str.empty()) sub.start = parseTime(start_str);
        if (!end_str.empty()) sub.end = parseTime(end_str);
        if (!text.empty()) sub.text = text;
        cout << "Субтитр обновлён.\n";
    }
    void remove(int idx) {
        if (idx<1 || idx>(int)subs.size()) { cout << "Неверный номер.\n"; return; }
        subs.erase(subs.begin()+idx-1);
        renumber();
        cout << "Субтитр удалён.\n";
    }
};

int main(int argc, char* argv[]) {
    if (argc < 2) {
        cout << "Использование: srt_editor [--shift мс] [--check] [--output file] файл.srt\n";
        return 1;
    }
    string filename;
    int shift_ms = 0;
    bool check_only = false;
    string output_file;
    for (int i=1; i<argc; ++i) {
        string arg = argv[i];
        if (arg == "--shift" && i+1 < argc) {
            shift_ms = stoi(argv[++i]);
        } else if (arg == "--check") {
            check_only = true;
        } else if (arg == "--output" && i+1 < argc) {
            output_file = argv[++i];
        } else {
            filename = arg;
        }
    }
    if (filename.empty()) {
        cout << "Не указан файл.\n";
        return 1;
    }
    Editor editor;
    if (!editor.load(filename)) {
        cout << "Ошибка загрузки.\n";
        return 1;
    }
    if (check_only) {
        auto errs = editor.validate();
        if (errs.empty()) cout << "Субтитры корректны.\n";
        else {
            cout << "Ошибки:\n";
            for (const auto& e : errs) cout << "  " << e << "\n";
        }
        return 0;
    }
    if (shift_ms != 0) {
        editor.shift(shift_ms);
        editor.save(output_file.empty() ? filename : output_file);
        return 0;
    }

    cout << "Редактор субтитров. Введите help для списка команд.\n";
    string cmd;
    while (true) {
        cout << "> ";
        getline(cin, cmd);
        if (cmd.empty()) continue;
        vector<string> parts;
        stringstream ss(cmd);
        string word;
        while (ss >> word) parts.push_back(word);
        if (parts.empty()) continue;
        string op = parts[0];
        if (op == "quit") break;
        else if (op == "help") {
            cout << "list, show <номер>, add, edit <номер> [время_начала] [время_конца] [текст], delete <номер>, shift <мс>, renumber, validate, save, quit\n";
        }
        else if (op == "list") {
            editor.list();
        }
        else if (op == "show") {
            if (parts.size() < 2) { cout << "Укажите номер.\n"; continue; }
            int idx = stoi(parts[1]);
            if (idx<1 || idx>(int)editor.subs.size()) { cout << "Неверный номер.\n"; continue; }
            auto& sub = editor.subs[idx-1];
            cout << "#" << sub.index << "  " << formatTime(sub.start) << " --> " << formatTime(sub.end) << "\n" << sub.text << "\n";
        }
        else if (op == "add") {
            string start, end, text;
            cout << "Начало (HH:MM:SS,mmm): "; getline(cin, start);
            cout << "Конец (HH:MM:SS,mmm): "; getline(cin, end);
            cout << "Текст (введите END для завершения):\n";
            string line;
            while (getline(cin, line)) {
                if (line == "END") break;
                text += line + "\n";
            }
            editor.add(start, end, text);
        }
        else if (op == "edit") {
            if (parts.size() < 2) { cout << "Укажите номер.\n"; continue; }
            int idx = stoi(parts[1]);
            string start, end, text;
            cout << "Новое начало (оставьте пустым): "; getline(cin, start);
            cout << "Новый конец (оставьте пустым): "; getline(cin, end);
            cout << "Новый текст (оставьте пустым): "; getline(cin, text);
            editor.edit(idx, start, end, text);
        }
        else if (op == "delete") {
            if (parts.size() < 2) { cout << "Укажите номер.\n"; continue; }
            int idx = stoi(parts[1]);
            editor.remove(idx);
        }
        else if (op == "shift") {
            if (parts.size() < 2) { cout << "Укажите сдвиг в мс.\n"; continue; }
            int ms = stoi(parts[1]);
            editor.shift(ms);
        }
        else if (op == "renumber") {
            editor.renumber();
        }
        else if (op == "validate") {
            auto errs = editor.validate();
            if (errs.empty()) cout << "OK\n";
            else {
                cout << "Ошибки:\n";
                for (const auto& e : errs) cout << "  " << e << "\n";
            }
        }
        else if (op == "save") {
            string fname = (parts.size()>1) ? parts[1] : "";
            if (editor.save(fname)) cout << "Сохранено.\n";
            else cout << "Ошибка сохранения.\n";
        }
        else {
            cout << "Неизвестная команда.\n";
        }
    }
    return 0;
}
