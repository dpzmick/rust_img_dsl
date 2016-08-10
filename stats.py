import pandas

names = ['image', 'time']

jitresults     = pandas.read_csv('jit.tsv', sep='\t', header=None, names=names)
print 'number of jit results:', len(jitresults.index)

constructtimes = jitresults[jitresults.image == 'construction']
compiletimes   = jitresults[jitresults.image == 'compilation']

print 'avg construction time:', constructtimes.mean()['time']
print 'avg compile time:', compiletimes.mean()['time']

jitrelevant   = jitresults[(jitresults.image != 'construction') & (jitresults.image != 'compilation')]
jitgroup      = jitrelevant.groupby('image')
jitmeans      = jitgroup.mean()
jitavgruntime = jitmeans.mean()['time']

print 'average jit runtime:', jitavgruntime

print

nativeresults    = pandas.read_csv('native.tsv', sep='\t', header=None, names=names)
print 'number of native results:', len(nativeresults.index)

nativegroup      = nativeresults.groupby('image')
nativemeans      = nativegroup.mean()
nativeavgruntime = nativemeans.mean()['time']

print 'average native runtime:', nativeavgruntime

print

print 'jit win:', nativeavgruntime - jitavgruntime
