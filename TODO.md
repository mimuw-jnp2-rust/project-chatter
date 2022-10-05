Notatki, ale jeszcze je troche przemyślałem, zagadnienia w kolejności dodawania:


1. Mapa userow indeksowana userName (janek)
- [ ] Wkladamy userow do mapy userow dopiero po otrzymaniu z WS ich nicka (nick zawarty w JSON)
2. Heartbeat (janek)
- [ ] struktura reprezentująca usera (WSClient) otrzymuje pole bool: isAlive
- [ ] dodajemy endpoint /heartbeat, w parametrze username, po zawolaniu ustawia zadanemu userowi isAlive na true
- [ ] dodatkowy wątek sprawdzający isAlive == true, jeśli nie to zamykamy połączenie WS, wysyłamy "user has left the char czy cos"
- [ ] jeśli zawołanie /heartbit sie nie powiedzie (timeout) - client informuje o zawieszeniu połączenia
3. Pokoje (kacper)
- [ ] robimy mapę key: nazwa pokoju, value: struct reprezentujący pokój
- [ ] zawartość structa reprezentującego pokój: tablica z nickami
- [ ] do endpointa /send (lub /post, nie pamietam, ten od wysyłania) dodajemy nazwe docelowego pokoju w postaci nowego pola common::Msg
- [ ] wywołanie handlera send sprawdza do którego pokoju ma wysłać wiadomosci i wysyla do wszystkich userów
- [ ] Rejestracja pokoju odbywa sie poprzez endpoint /createroom (w parametrze nazwa nowego pokoju), metoda http wołana gdy w prompt wybieramy nowy pokoj LUB (ale to zostawiam twojej dyspozycji realizacje tej rejestracji, jest wiele sposobow)
- podczas podłączania WS do jsona z pkt 1. (ten z nickiem) dodajemy również pożądany pokój
4. Msgs archiwum
- [ ] do kazdego pokoju dodajemy tablice z archiwalnymi msgami
- [ ] w momencie podłączenia, wysyłamy po ws wszystkie dotychczasowe wiadomości

ELO