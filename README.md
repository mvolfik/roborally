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
pohybuje po herním plánu. Cílem hry je projet vyznačené checkpointy na mapě v určeném pořadí.
Nicméně ovládání robota po továrně není jen tak - v každém kole každý hráč svému robotovi
nastaví 5 kroků (tzv. "registrů"), které má robot vykonat. Tyto kroky pak ale vykonávají
všichni hráči současně, v pořadí dle vzdálenosti od "prioritní antény", a na herním plánu
se navíc po každém registru aktivují pohyblivé prvky.

Tzn. nejprve hráč, který je nejblíže anténě, vykoná svůj první krok, po něm hráč který je
druhý v pořadí, atd. Poté se aktivují prvky na plánu - posuvné pásy, push panely, otáčecí
plošiny, a nakonec roboti a lasery na mapě vystřelí (hra spouští animace pouze těch střel,
které některého z robotů zasáhnou). Pokud je některý z robotů v tuto chvíli, na samotném
konci vykonávání jednotlivého registru, na checkpointu, který má projet jako následující,
je mu započítán. Poté následuje stejným způsobem další registr - opět v (velmi
pravděpodobně už trochu pozměněném) pořadí dle vzdálenosti od antény.

Během pohybu je také možné z mapy vypadnout - vyjetím buď mimo okraj mapy, nebo do některé
z děr uvnitř mapy. V tom případě se hráč "rebootuje" - robot se znovu objeví na "reboot tokenu", který je zpravidla uprostřed mapy, a až do konce kola se už jeho kroky
nevykonávají.

Hráči kroky robotům nastavují ("programují") pomocí karet. Každý hráč začíná s 20 kartami,
které má zamíchané na svém dobíracím balíčku. Z tohoto balíčku si vždy na začátku kola
lízne 9 karet, z nichž poté vybírá, které "naprogramuje" do registrů. Zbylé karty (a po
vykonání i ty použité) poté odloží na svůj vlastní odkládací balíček. Ve chvíli, kdy hráči
dojdou karty na jeho dobíracím balíčku, zamíchá svůj odkládací balíček. Kromě pohybových
karet ale během hry hráči přibývají do balíčku také SPAM karty - hráč dostává jednu za každou
střelu, která ho zasáhne, a dvě pokaždé, když se rebootuje. Tyto karty je možné také nastavit
do některého z registrů. Ve chvíli, kdy se má SPAM karta vykonat jako krok, se tato karta
odloží (hráč se tak této jedné SPAM karty definitivně zbaví), a lízne se vrchní karta z
dobíracího balíčku hráče. Tato "náhodná" karta se pak vykoná.

## Architektura

Samotnou herní simulaci vykonává výhřadně server, a připojeným hráčům/klientům posílá pouze
data potřebná k zobrazení aktuálního stavu. Webový klient se k vybrané hře připojí přes
WebSocket. Přes tento WebSocket posílá každá strana `enum` `ServerMessage`, respektive
`ClientMessage`, zakódovaný do kompaktních binárních zpráv pomocí knihovny messagepack.

Hra rozlišuje tři fáze:

- `Programming`: hráči programují své kroky pro další kolo. Zde se čeká, až každý hráč v
  klientovi "uloží program", tzn. odešle informaci o zvolených kartách
- `Moving`: vykonávají se kroky dle zvolených karet a prvků na herním plánu. Server celou
  tuto fázi projde od začátku do konce synchronně, tzn. prakticky okamžitě. V průběhu toho
  ukládá seznam (`Vec` v Rustu) `PlayerGameStateView` pro každého hráče, kterou na konci fáze
  odešle připojeným klientům pro zobrazení (animaci) dílčích kroků.
- `HasWinner`: hra skončila, některý z hráčů úspěšně prošel všechny checkpointy. Hra je
  odstraněna ze seznamu na hlavní stránce (kromě toho jsou "zahazovány" také hry, do kterých
  se nikdo po dobu 5 minut nepřipojil) a nelze se do ní připojit. Po odpojení všech hráčů
  nezůstanou žádné reference na `Arc` (atomic reference-counted pointer), který hru drží,
  a hra je tak smazána.

`ServerMessage` má tedy (kromě informativní `Notice`) 2 varianty, které poskytují informace
o stavu hry:

- `State` je informací o nejaktuálnějším stavu hry - tento stav má vždy herní fázi
  `Programming` (případně pak `HasWinner`)
- `AnimatedStates` obsahuje seznam kroků, které lze na klientovi procházet jako animaci
  `Moving` fáze. Jednotlivé kroky mohou obsahovat jednak animace (např. průlety střel),
  a také samotný stav s pozicemi hráčů, kartami pro aktuální registr atd

`ClientMessage` má pouze jedinou variantu: `Program`, tedy informaci o zvolených
("naprogramovaných") kartách pro další kolo.

## Technologie

Velmi silně staticky typovaný, relativně nízkoúrovňový jazyk Rust se může na první pohled zdát
jako ne příliš vhodný pro simulaci deskové hry, která není nijak výpočetně náročná, a člověk
by mohl očekávat potřebu flexibility ohledně datových typů. Nicméně já jsem tento jazyk zvolil
právě kvůli silné typovosti. Moje předchozí zkušenosti jsou zejména s jazyky Python a
TypeScript, u kterých jsem vždy vnímal, že tyto jazyky nevznikly okolo striktně staticky
typovaných dat, ale typy byly naopak až později dodány, a ne vše tak lze pohodlně vyjádřit.
Schopnosti Rustu (zejména pak [algebraické datové typy][adt], tzn. enum varianty s obsaženými
daty, a pokročilý pattern-matching) mi však umožnily pohodlně reprezentovat stav hry a
vlastnosti všech políček mapy, a mít jistotu že v každém místě kódu je ošetřena každá varianta.

[adt]: https://en.wikipedia.org/wiki/Algebraic_data_type

### `/backend/roborally-server`

Serverový kód díky výše zmíněnému považuji za poměrně čistý. Jeho "nejošklivější" částí je
dlouhá funkce, která evaluuje pohybovou fázi hry. Nicméně tato funkce obsahuje pouze `loop` s
`match` konstrukcí, jejíž každá větev definuje akce pro jednotlivé fáze vykonávání (provedení
akcí registrů, posun na pásech, atd.). Každou z těchto větví lze však číst odděleně od zbytku,
kód tak zůstává přehledný.

### `/backend/roborally-structs`, `/backend/roborally-frontend-wasm`

Pro "přebírání" stavu od serveru na straně klienta jsem se rozhodl použít možnosti kompilace
Rustu do WebAssembly. Všechny struktury, které jsou předávány mezi serverem a klientem jsou
v oddělené `crate` (Rust balíčku) `roborally-structs`. WebAssembly tak přijatou binární
zprávu deserializuje do přesně stejných struktur, které server odesílá. Pomocí `wasm-bindgen`
jsou pak z těchto struktur do prezentačního Javascriptu předávány hodnoty, které už klient
nemusí téměř vůbec upravovat (např. potřebné transformace pro dílky na mapě nebo roboty jsou
ukládány ve struktuře `Effects`, která pak vygeneruje adekvátní CSS deklaraci). Díky tomu, že
`wasm-bindgen` generuje i Typescriptové deklarace, vygenerované přístupy do Rustové
WebAssembly lze používat pohodlně a bez nebezpečí překlepů apod.

Jediným problémem, který se v tomto přístupu ukázal, jsou omezení, která `wasm-bindgen` zatím
má - knihovna nepodporuje mj. předávání `Vec` jako `Array` (pomocí makra jsou tak pro potřebné
typy generovány manuální konverze včetně typescriptových deklarací), volání metod na `enum` hodnotách a `enum` varianty s obsaženými dalšímy daty - okolo některých typů jsou tak potřeba
wrapper struktury, které tyto omezení obchází. Část z tohoto kódu také bohužel nemohla být
přímo v `crate` `roborally-frontend-wasm`, ale v `roborally-structs` kvůli přístupu k private
atributům. `roborally-structs` tak má compile-time `features` `client` a `server`, kterými
jsou některé potřebné části kódu podmíněny. Každá strana kódu tak tuto sdílenou knihovnu může
importovat pouze s potřebnými částmi.

### `/roborally-frontend`

Single-page webová aplikace napsaná pomocí frameworku Svelte neobsahuje mnoho opakovaně
využitelného kódu - je zde tak jedna vnější komponenta pro vnější aplikaci, která obsahuje
hlavní menu s výběrem her. Po připojení do hry zabere celou obrazovku komponenta `Game`,
která se stará o udržování připojení k serveru, správu aktuálního stavu hry (zejména
"animaci" seznamu stavů přijatých ze serveru během `Moving` fáze) a zobrazování relevantních
menu k aktuálnímu stavu (interface k programování registrů vs. pouhé zobrazení vybraných karet
apod.). Díky tomu, že stav do prezentačních atributů "překládá" WebAssembly modul, jak je
vysvětleno výše, frontend se může zaměřit opravdu jen na prezentaci, která tak nemusí být
propletena s herní logikou ani překládáním definice mapy na zobrazované dílky.
