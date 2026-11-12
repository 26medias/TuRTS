pre={'P1':(4,6),'P2':(5,8),'P3':(2,4)}
mp1={'X1':(3,5),'X2':(4,7),'X4':(6,10),'X5':(4,6),'X6':(4,6),'X7':(2,4),'X8':(3,4),'X9':(2,3),'X10':(2,3),'X11':(6,9),'X12':(6,14),'X13':(3,5),'X14':(4,7),'X15':(8,12),'X17':(2,3),'X18':(4,6),'X19':(3,5),'X20':(12,20),'X21':(3,5),'X22':(2,3),'X23':(2,2)}
assets=(8,16); gates=(20,35)
s=lambda d:(sum(a for a,b in d.values()),sum(b for a,b in d.values()))
print('pre',s(pre)); print('mp1 build',s(mp1))
lo,hi=s(mp1); print('mp1+gates',lo+gates[0],hi+gates[1],'with assets',lo+gates[0]+assets[0],hi+gates[1]+assets[1])
cut={k:v for k,v in mp1.items() if k not in('X7','X8','X9','X10','X18','X17')}
cl,ch=s(cut); print('cut build',cl,ch,'with gates',cl+gates[0],ch+gates[1])
mp2={'bot':(10,16),'botc':(6,10),'sight':(4,8),'idx':(3,5),'lobby':(3,5),'instr':(8,14),'audit':(2,2)}
a,b=s(mp2); print('mp2',a,b,'gates',a+15,b+25)
