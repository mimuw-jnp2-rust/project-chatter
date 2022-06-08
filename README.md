# Chatter

## Autorzy:
	Kacper Kramarz-Fernandez,
	mgr inż. (XD) Jan Zembowicz

## Opis:

	Projekt stanowi system prostego czatu o pojedynczym pokoju i dowolnej liczby klientów.
	Dzieli się on na:
	- Klient - umożliwia nadawanie wiadomości oraz odbieranie ich i wyświetlanie 
	- Serwer - stanowi pokój, odbiera wiadomość wysłaną przez klienta i rozsyła ją do wszystkich pozostałych

## Funkcje: (<i>"ponieważ funkcjonalność to stan, w którym coś funkcjonuje"</i> - Jan "prof. Miodek" Zembowicz)
	
	- Klient: dwuprocesowy program:
		1. Wysyłanie wiadomości za pomocą requesta typu GET, wiadomość wysyłana w parametrze URL wraz z nią timestamp
		2. Odbieranie wiadomości asynchronicznie za pomocą klienta WebSocket i wyświetlenie ich w terminalu
	- Serwer: prawdopodobnie dwuwątkowy
		1. Odbiera requesty HTTP od pojedynczego klienta
		2. Wysyła odebraną wiadomość po WebSocket do wszystkich obecnie połączonych klientów

## Podział pracy
	Uwaga: zaproponowany podział jest płynny!
	
	Początkowo
	- Jan: podstawowa komunikacja sieciowa
	- Kacper: przeniesienie zaimplementowanych mechanizmów przez Jan na kod o wyższej jakości, zgodny z dobrymi praktykami tworzenia oprogramowania w języku Rust
	
	Etap 2:
	- Jan: system heartbeat + indeksowanie klientów WS po username
	- Kacper: Wiele pokoi + msg storage 


## Podział projektu na części
    Sprint 1
        - wysyłanie i odbieranie podstawowych komunikatów z timestampem,
    Sprint 2
        - wiele pokoi, historia wiadomosci, heartbeat, polaczenie "displayera" za pomocaą username

## Biblioteki i zależności:
	
	- Wysyłanie requestów HTTP: reqwest
	- Serwer HTTP: ??? (prawdopodonie punktem wyjścia będzie jeden z przykładów biblioteki `hyper`)
	- Serwer i klient WebSocket: ???


## Struktura repo
	- server - folder z projektem servera
	- client - folder z projektem clienta
	- common - wspolny kod dla obu projektow

## Uruchomienie

	$ cargo run

	W katalogu odpowiednio dla serwera oraz klienta
