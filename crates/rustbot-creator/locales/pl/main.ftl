## Ekran powitalny.
welcome-tagline = Twórz boty na Discorda na swoim komputerze: bez kodu albo z pełną kontrolą nad kodem.
new-project = Nowy projekt…
open-project = Otwórz projekt…
pick-project-folder = Wybierz folder projektu bota
error-open = Nie udało się otworzyć projektu: { $error }
error-create = Nie udało się utworzyć projektu: { $error }

## Panele.
panel-project = Projekt
panel-editor = Edytor
panel-properties = Właściwości
panel-logs = Logi
panel-problems = Problemy
tree-commands = Komendy
tree-events = Zdarzenia
properties-empty = Zaznacz węzeł na płótnie, aby go edytować.
editor-empty = Otwórz komendę albo zdarzenie z drzewa projektu.
logs-empty = Tu pojawi się wyjście bota, gdy go uruchomisz.
logs-clear = Wyczyść
problems-empty = Brak problemów.

## Pasek stanu.
status-stopped = Bot zatrzymany
status-language = Polski

## Problemy w grafie.
problem-no-trigger = Graf nie ma wyzwalacza, np. komendy z ukośnikiem.
problem-several-triggers = Graf ma więcej niż jeden wyzwalacz.
problem-unknown-node-type = Nieznany typ węzła { $type }. Czy brakuje wtyczki?
problem-unknown-pin = Kabel przy { $pin } prowadzi donikąd.
problem-wrong-direction = Kabel przy { $pin } biegnie w złą stronę.
problem-kind-mismatch = Nie można połączyć { $from } z { $to }.
problem-several-sources = Do { $pin } prowadzi więcej niż jeden kabel.
problem-missing-prop = Pole „{ $prop }” jest puste.
problem-unknown-prop = Pole „{ $prop }” nie należy do tego węzła.
problem-wrong-prop-type = Pole „{ $prop }” ma wartość złego rodzaju.
problem-template-unclosed = W polu „{ $prop }” jest {"{"} bez zamknięcia.
problem-template-stray = W polu „{ $prop }” jest {"}"} bez {"{"}. Dla klamry napisz {"}}"}.
problem-template-empty = W polu „{ $prop }” jest pusta zmienna, np. {"{}"}.
problem-unknown-variable = Pole „{ $prop }” używa { $path }, które nie istnieje.

## Płótno grafu.
canvas-add = Dodaj węzeł
picker-search = Szukaj węzłów…
picker-empty = Żaden węzeł tu nie pasuje.
error-save = Nie udało się zapisać { $file }: { $error }

## Wbudowane węzły, ich piny i pola.
node-core-slash_command = Komenda z ukośnikiem
node-core-reply = Odpowiedz
pin-then = potem
pin-run = uruchom
prop-core-slash_command-name = Nazwa komendy
prop-core-slash_command-description = Opis
prop-core-reply-content = Wiadomość

## Panel właściwości i ustawienia bota.
props-variables = Zmienne: { $list }
bot-settings = Ustawienia bota
bot-token = Token bota
bot-token-placeholder = Wklej token z Discord Developer Portal
bot-token-checking = Sprawdzam systemowy pęk kluczy…
bot-token-saved = Token jest zapisany w systemowym pęku kluczy.
bot-token-missing = Nie zapisano jeszcze tokenu.
save = Zapisz
bot-test-server = ID serwera testowego
bot-test-server-hint = Podczas pracy komendy rejestrują się na tym serwerze i pojawiają się od razu.
bot-test-server-saved = Zapisano.
error-keyring = Nie można użyć systemowego pęku kluczy: { $error }
error-server-id = „{ $id }” to nie jest ID serwera: składa się z samych cyfr.

## Uruchamianie i zatrzymywanie.
run = Uruchom
stop = Zatrzymaj
cancel = Anuluj
status-running = Bot działa
run-checking = Sprawdzam projekt…
run-has-problems = Projekt ma problemy. Najpierw je popraw; lista jest w zakładce Problemy.
run-cancelled = Anulowano.
trust-title = Zbudować projekt z zewnątrz?
trust-detail = Budowanie uruchamia kod na tym komputerze: crate'y, od których projekt zależy, i { $files } plików jego własnego kodu w src/. Buduj go tylko wtedy, gdy ufasz jego źródłu.
trust-build = Ufam, zbuduj
gcc-title = Zainstalować kompilator C?
gcc-detail = Boty buduje się za pomocą gcc, którego w tym systemie nie ma. Zainstaluje go menedżer pakietów systemu i poprosi o hasło.
gcc-install = Zainstaluj gcc
gcc-installing = Instaluję gcc…
gcc-unknown-system = Nie znaleziono obsługiwanego menedżera pakietów. Zainstaluj gcc samodzielnie i naciśnij Uruchom jeszcze raz.
gcc-failed = Nie udało się zainstalować gcc. Powód jest w liniach powyżej.
rust-title = Zainstalować Rusta?
rust-detail = Boty buduje Rust { $version }, którego aplikacja instaluje we własnym folderze: około 300 MB do pobrania. Twój własny Rust nie zostanie zmieniony.
rust-install = Zainstaluj Rusta
rust-installing = Instaluję Rusta…
run-generating = Generuję kod…
run-building = Buduję bota…
run-starting = Uruchamiam bota…
run-started = Bot działa.
run-stopping = Zatrzymuję bota…
run-exited = Bot się zatrzymał ({ $status }).
run-no-token = Bot nie ma jeszcze tokenu. Kliknij puste miejsce na płótnie i dodaj go w ustawieniach bota w panelu Właściwości.
error-no-home = Nie znam folderu domowego, więc aplikacja nie ma gdzie zainstalować Rusta.
error-trust = Nie udało się oznaczyć projektu jako zaufanego: { $error }
error-install = Nie udało się zainstalować Rusta: { $error }
error-codegen = Nie udało się wygenerować kodu: { $error }
error-build = Budowanie się nie powiodło: { $error }
error-start = Nie udało się uruchomić bota: { $error }
