Exercise 1 - Theory questions
-----------------------------

### Concepts

What is the difference between *concurrency* and *parallelism*?
> Concurrency er oppsettet av individuelle prosesser. Parallellisme er når prosessene kjører sammtidig. 

What is the difference between a *race condition* and a *data race*? 
> race condition er når resultatet er avhening av ordning i tid av kildene

Et datarace er en spesifikk type race condition når to eller fler tråder bruker delte variable sammtidig, og minst en av trådene skriver til variabelen.
 
*Very* roughly - what does a *scheduler* do, and how does it do it?
> Scheduler allokere CPU tid til forskjellige tråder, og har oversikt over hvem som utføres til hvilken tid.


### Engineering

Why would we use multiple threads? What kinds of problems do threads solve?
> Vi bruker tråder for å løse forskjellige problemer 'til samme tid'.
> Tråder brukes i for eksempel heis lab.

Some languages support "fibers" (sometimes called "green threads") or "coroutines"? What are they, and why would we rather use them over threads?
> Fiber er user-spaced tråer som er planlagt av et runtime bibliotek, eler virituell maskin og ikke av operativsystemet selv.
> Fibre er å foretrekke noen ganger da de har mindre overhead. De gjør også livet lettere når en designer sanntidsprogrammer ved å for eksempel unngå race conditions og deadlocks.


Does creating concurrent programs make the programmer's life easier? Harder? Maybe both?
> Sanntidsprogrammer gjør ikke arbeidet lettere for programmeren, da det er flere maskiner som skal sammarbeide om samme oppgave. Det er dog nødvendig for å løse noen typer problemer og sikre oppetid, da en maskin kan ta over arbeidet til en annen ved bortfall.

What do you think is best - *shared variables* or *message passing*?
> Jeg tror de to løsningene kan brukes til hvert sin formål. Delte variable er gunstig når oppgaver trenger hyppig tilgang til felles data. Men man må sikre nøyaktig synronisering for å unngå problemer som race condition.
> Message passing passer best for sanntidsprogrammer som ikke nødvendigvis er avhengig av globale variable til samme tid, og synkronisering er noe mer løsluppet. 


