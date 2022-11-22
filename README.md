# roborally

Web version of the RoboRally board game – high school graduation project

[Rules download link](https://www.hasbro.com/common/documents/60D52426B94D40B98A9E78EE4DD8BF94/3EA9626BCAE94683B6184BD7EA3F1779.pdf)

## Build

```sh
git archive --prefix roborally/ -o source-code.tar.gz HEAD && \
git archive --prefix roborally/ -o source-code.zip HEAD && \
env DOCKER_BUILDKIT=1 docker build -t roborally:dev .
```

Run with `docker run --rm -p 80:80 -e PORT=80 roborally:dev`

## Popis hry

Roborally je tahová strategická desková hra. Každý hráč ovládá jednoho robota, který se
pohybuje po herním plánu. Cílem hry je projet vyznačené checkpointy na mapě v určeném
pořadí. Nicméně ovládání robota po továrně není jen tak - v každém kole každý hráč svému
robotovi nastaví 5 kroků (tzv. "registrů"), které má robot vykonat. Tyto kroky pak ale
vykonávají všichni hráči současně, v pořadí dle vzdálenosti od "prioritní antény", a na
herním plánu se navíc po každém registru aktivují pohyblivé prvky.

Tzn. nejprve hráč, který je nejblíže anténě, vykoná svůj první krok, po něm hráč který je
druhý v pořadí, atd. Poté se aktivují prvky na plánu - posuvné pásy, push panely, otáčecí
plošiny, a nakonec roboti a lasery na mapě vystřelí (hra spouští animace pouze těch střel,
které některého z robotů zasáhnou). Pokud je některý z robotů v tuto chvíli, na samotném
konci vykonávání jednotlivého registru, na checkpointu, který má projet jako následující,
je mu započítán. Poté následuje stejným způsobem další registr - opět v (velmi
pravděpodobně už trochu pozměněném) pořadí dle vzdálenosti od antény.

Během pohybu je také možné z mapy vypadnout - vyjetím buď mimo okraj mapy, nebo do některé
z děr uvnitř mapy. V tom případě se hráč "rebootuje" - robot se znovu objeví na "reboot
tokenu", který je zpravidla uprostřed mapy, a až do konce kola se už jeho kroky
nevykonávají.

Hráči kroky robotům nastavují ("programují") pomocí karet. Každý hráč začíná s 20 kartami,
které má zamíchané na svém dobíracím balíčku. Z tohoto balíčku si vždy na začátku kola
dobere 9 karet, z nichž poté vybírá, které "naprogramuje" do registrů. Zbylé karty (a po
vykonání i ty použité) poté odloží na svůj vlastní odkládací balíček. Ve chvíli, kdy hráči
dojdou karty na jeho dobíracím balíčku, zamíchá svůj odkládací balíček. Kromě pohybových
karet ale během hry hráči přibývají do balíčku také SPAM karty - hráč dostává jednu za
každou střelu, která ho zasáhne, a dvě pokaždé, když se rebootuje. Tyto karty je možné
také nastavit do některého z registrů. Ve chvíli, kdy se má SPAM karta vykonat jako krok,
se tato karta odloží (hráč se tak této jedné SPAM karty definitivně zbaví), a dobere se
vrchní karta z dobíracího balíčku hráče. Tato "náhodná" karta se pak vykoná.

## Architektura

Samotnou herní simulaci vykonává výhradně server, a připojeným hráčům/klientům posílá
pouze data potřebná k zobrazení aktuálního stavu. Webový klient se k vybrané hře připojí
přes WebSocket. Přes tento WebSocket posílá každá strana `enum` `ServerMessage`,
respektive `ClientMessage`, zakódovaný do kompaktních binárních zpráv pomocí knihovny
messagepack.

`ServerMessage` má následující varianty:

- `Notice` - krátká textové hláška, kterou má klient zobrazit - typicky chybové hlášky
- `GameLog` - obsahuje chybové i jiné technické zprávy z vykonávání karet
- `GeneralState` - informace o připojených hráčích a o tom, na co se zrovna čeká, nebo co
  server počítá
- `ProgrammingState` - informace o tom, jaké karty má hráč k dispozici pro další kolo
  (případně jaké už nastavil), a na které hráče se ještě čeká
- `AnimatedState` - obsahuje jeden krok animace pohybu hráčů - těchto zpráv server typicky
  pošle několik v řadě za sebou, až do konce kola, případně potřeby čekat na nějaké
  rozhodnutí některého hráče

Herní klient aktuálně po WebSocketu posílá pouze informace o vybraných kartách. Své jméno
a herní židli oznamuje v query stringu při navázání spojení.

## Technologie

Velmi silně staticky typovaný, relativně nízkoúrovňový jazyk Rust se může na první pohled
zdát jako ne příliš vhodný pro simulaci deskové hry, která není nijak výpočetně náročná, a
člověk by mohl očekávat potřebu flexibility ohledně datových typů. Nicméně já jsem tento
jazyk zvolil právě kvůli silné typovosti. Moje předchozí zkušenosti jsou zejména s jazyky
Python a TypeScript, u kterých jsem vždy vnímal, že tyto jazyky nevznikly okolo striktně
staticky typovaných dat, ale typy byly naopak až později dodány, a ne vše tak lze pohodlně
vyjádřit. Schopnosti Rustu (zejména pak [algebraické datové typy][adt], tzn. enum varianty
s obsaženými daty, a pokročilý pattern-matching) mi však umožnily pohodlně reprezentovat
stav hry a vlastnosti všech políček mapy, a mít jistotu že v každém místě kódu je ošetřena
každá varianta.

[adt]: https://en.wikipedia.org/wiki/Algebraic_data_type

### `/backend/roborally-server`

Serverový kód byl kvůli implementaci skriptovatelných karet (a také díky zkušenostem z
první verze kódu) téměř celý přepsán. Je zde totiž důležitá hranice mezi statickými daty
hry (počet hráčů, definice karet, herní mapa) a aktuálním stavem, který můžou právě i
karty měnit. V modulu `game.rs` je tak implementované řízení vykonávání pohybu hráčů v
jednotlivých krocích, a struktura `GameState` v modulu `game_state.rs` obsahuje dynamická
stavová data.

Při vytváření hry server vytvoří ze skriptů karet AST, a pro každou kartu vytvoří scope,
do kterého si karta může ukládat libovolná data. Zároveň je do tohoto scope přidaný
objekt, pomocí jehož metod může karta hru ovládat, tedy vykonávat pohyby. Definice tohoto
externího API je v souboru `rhai_api.rs`.

Struktura `Game` je neměnná, a lze k ní tak přistupovat jednoduše skrz sdílený
reference-counted pointer. Naopak přístup k `GameState` je potřeba ošetřit zámkem. Tento
zámek nelze udržovat ve struktuře `Scope` odemčený, API dostupné kartám ho tedy bohužel
musí pro každý vykonávaný pohyb odemknout a zamknout. Pro zbylé pohybové fáze si už zámek
ale odemkneme pouze jednou a vykonáme vše najednou.

### `/backend/roborally-structs`, `/backend/roborally-frontend-wasm`

Pro "přebírání" stavu od serveru na straně klienta jsem se rozhodl použít možnosti
kompilace Rustu do WebAssembly. Všechny struktury, které jsou předávány mezi serverem a
klientem jsou v oddělené `crate` (Rust balíčku) `roborally-structs`. WebAssembly tak
přijatou binární zprávu deserializuje do přesně stejných struktur, které server odesílá.
Pomocí `wasm-bindgen` jsou pak z těchto struktur do prezentačního Javascriptu předávány
hodnoty, které už klient nemusí téměř vůbec upravovat (např. potřebné transformace pro
dílky na mapě nebo roboty jsou ukládány ve struktuře `Effects`, která pak vygeneruje
adekvátní CSS deklaraci). Díky tomu, že `wasm-bindgen` generuje i Typescriptové deklarace,
vygenerované přístupy do Rustové WebAssembly lze používat pohodlně a bez nebezpečí
překlepů apod.

Jediným problémem, který se v tomto přístupu ukázal, jsou omezení, která `wasm-bindgen`
zatím má - knihovna nepodporuje mj. předávání `Vec` jako `Array` (pomocí makra jsou tak
pro potřebné typy generovány manuální konverze včetně typescriptových deklarací), volání
metod na `enum` hodnotách a `enum` varianty s obsaženými dalšímy daty - okolo některých
typů jsou tak potřeba wrapper struktury, které tyto omezení obchází. Část z tohoto kódu
také bohužel nemohla být přímo v balíčku `roborally-frontend-wasm`, ale v
`roborally-structs` kvůli přístupu k private atributům. `roborally-structs` tak má
compile-time `features` `client` a `server`, kterými jsou některé potřebné části kódu
podmíněny. Každá strana kódu tak tuto sdílenou knihovnu může importovat pouze s potřebnými
částmi.

### `/roborally-frontend`

Single-page webová aplikace napsaná pomocí frameworku Svelte neobsahuje mnoho opakovaně
využitelného kódu - je zde tak jedna vnější komponenta pro vnější aplikaci, která obsahuje
hlavní menu s výběrem her. Po připojení do hry zabere celou obrazovku komponenta `Game`,
která se stará o udržování připojení k serveru, správu aktuálního stavu hry (zejména
"animaci" seznamu stavů přijatých ze serveru během pohybové fáze) a zobrazování
relevantních menu k aktuálnímu stavu (interface k programování registrů vs. pouhé
zobrazení vybraných karet apod.). Díky tomu, že stav do prezentačních atributů "překládá"
WebAssembly modul, jak je vysvětleno výše, frontend se může zaměřit opravdu jen na
prezentaci, která tak nemusí být propletena s herní logikou ani překládáním definice mapy
na zobrazované dílky.
