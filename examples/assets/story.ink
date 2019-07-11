Hours go by too quickly sometimes. Before you knew it your writing session had stretched into the little hours. 

+   [Look at your phone]
    -> phone
+   "Heck[!"]," you say to yourself. You wanted to get up early tomorrow! Or, well, today.
    + +     [Put on more coffee]
            No matter, might as well get some more work done.
            -> next_day
    + +     You head to your bed[]. Better some sleep than none, after all.
            -> next_day


=== next_day ===
FOLLOWING DAY

You awake, tired as ever.

+   {rauan} A reply from Rauan urges you to nab your coat and head for some coffee. -> coffee
+   {not rauan} No plans for today.[] Is this your life now?
    -> END


=== phone ===
You take a quick glance at your phone. Unread mail and twitter notifications are swiped away with a tired motion. Ah! A missed call and text from Rauan. 
+   You decide to deal with it tomorrow.
    -> next_day
+   [Text back] -> rauan 


=== rauan ===
"Sorry, didn't see your call," you text back, trusting they won't wake from it. "Still up for coffee tomorrow?" After that, you head to bed.
-> next_day


=== coffee ===
You spend your day with them, having a lovely time. 
-> END