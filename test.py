import matplotlib.pyplot as plt
import numpy as np

x = np.linspace(0, 1000, 100000)

a = 0.0001
b = 0.1
c = 0.0004701

def func1(w):
    return 1/(1-b*c*x**2)


def dB(H):
    return 20*np.log10(np.abs(H))

def cutOffFreq():
    return np.sqrt((1+10**(3/20))/(b*c))

H = dB(func1(x))

for i in range(0, len(H)):
    if H[i] <= -3:
        fc_index = i
        break

print(fc_index)

print(cutOffFreq())


plt.plot(x, H)
plt.plot(x[fc_index], H[fc_index], 'ro')
plt.ylim([-40, 40])
0
plt.show()